

/**
 * Helpers for the async support.
 */

public enum EventCode {
  None = 0,
  Subtask = 1,
  StreamRead = 2,
  StreamWrite = 3,
  FutureRead = 4,
  FutureWrite = 5,
  Cancel = 6,
}

public enum CallbackCode : uint
{
    Exit = 0,
    Yield = 1,
    // TODO:
    //#define TEST_CALLBACK_CODE_WAIT(set) (2 | (set << 4))
}

public partial class WaitableSet(int handle) : IDisposable
{
    public int Handle { get; } = handle;

    void Dispose(bool _disposing)
    {
        AsyncSupport.WaitableSetDrop(handle);
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~WaitableSet()
    {
        Dispose(false);
    }
}

public static class AsyncSupport
{
    private static class Interop
    {
        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[waitable-set-new]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static extern int WaitableSetNew();

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[waitable-join]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static extern void WaitableJoin(int waitable, int set);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[waitable-set-wait]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern int WaitableSetWait(int waitable, uint* waitableHandlePtr);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[waitable-set-poll]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern uint WaitableSetPoll(int waitable, uint* waitableHandlePtr);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[waitable-set-drop]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern void WaitableSetDrop(int waitable);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[context-set-0]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern void ContextSet(ContextTask* waitable);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[context-get-0]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern ContextTask* ContextGet();
    }

    public static WaitableSet WaitableSetNew() 
    {
        var waitableSet  = Interop.WaitableSetNew();
        return new WaitableSet(waitableSet );
    }

    public static unsafe void WaitableSetPoll(int waitableHandle) 
    {
        var error  = Interop.WaitableSetPoll(waitableHandle, null);
        if(error != 0)
        {
            throw new Exception($"WaitableSetPoll failed with error code: {error}");
        }
    }

    public static void Join(FutureWriter writer, WaitableSet set) 
    {
        Interop.WaitableJoin(writer.Handle, set.Handle);
    }

    public static void Join(FutureReader reader, WaitableSet set) 
    {
        Interop.WaitableJoin(reader.Handle, set.Handle);
    }

    // TODO: Revisit this to see if we can remove it.
    // Only allow joining to a handle directly when there is no waitable.
    public static void Join(int handle) 
    {
        Interop.WaitableJoin(handle, 0);
    }

    public unsafe static EventWaitable WaitableSetWait(WaitableSet set) 
    {
        uint* buffer = stackalloc uint[2];
        var eventCode = (EventCode)Interop.WaitableSetWait(set.Handle, buffer);
        return new EventWaitable(eventCode, buffer[0], buffer[1]);
    }

    public static void WaitableSetDrop(int handle) 
    {
        Interop.WaitableSetDrop(handle);
    }

    // The context that we will create in unmanaged memory and pass to context_set.
    // TODO: C has world specific types for these pointers, perhaps C# would benefit from those also.
    [StructLayout(LayoutKind.Sequential)]
    public struct ContextTask
    {
        public int Set;
        public int FutureHandle;
    }

    public readonly struct Event
    {
        public Event(int raw, int waitable, uint code)
        {
            Raw = raw;
            Waitable = waitable;
            Code = code;
            WaitableStatus = new WaitableStatus(code & 0xf);
        }

        public readonly int Raw;
        public readonly int Waitable;
        public readonly uint Code;

        public readonly EventCode EventCode => (EventCode)Raw;
        public readonly WaitableStatus WaitableStatus;
        public readonly uint WaitableCount => Code >> 4;
    }

    public static unsafe void ContextSet(ContextTask* contextTask)
    {
        Interop.ContextSet(contextTask);
    }

    public static unsafe ContextTask* ContextGet()
    {
        ContextTask* contextTaskPtr = Interop.ContextGet();
        if(contextTaskPtr == null)
        {
            throw new Exception("null context returned.");
        }
        return contextTaskPtr;
    }

    public static unsafe CallbackCode Callback(Event e, ContextTask* contextPtr, Action taskReturn)
    {
        // TODO: Looks complicated....
        if(PendingCallbacks.TryRemove((IntPtr)(contextPtr), out var tcs))
        {
            Marshal.FreeHGlobal((IntPtr)contextPtr);
            taskReturn();

            tcs.SetResult();
        }
        return CallbackCode.Exit;
    }

    // From the context pointer to the task.
    public static ConcurrentDictionary<IntPtr, TaskCompletionSource> PendingCallbacks = new ConcurrentDictionary<IntPtr, TaskCompletionSource>();
}

/**
 * Helpers for future support.
 */
public delegate ulong New();
public delegate uint StartRead(int handle, IntPtr buffer);
public delegate void DropReader(int handle);
public delegate void DropWriter(int handle);
public delegate uint Write(int handle, IntPtr buffer);

public struct FutureVTable
{
    public New New;
    public StartRead StartRead;
    public Write Write;
    public DropReader DropReader;
    public DropWriter DropWriter;
}

public struct TaskState
{
    //TODO: A copy of the go taskState, what else do we need?
    // channel     chan unit
    internal WaitableSet? WaitableSet;
    // pending     map[uint32]chan uint32
    // yielding    chan unit
    // pinner      runtime.Pinner
}


public static class FutureHelpers
{
    static TaskState state = new TaskState();

    /// Helper function to create a new read/write pair for a component model
    /// future.
    public static (FutureReader, FutureWriter) RawFutureNew(FutureVTable vtable)
    {
        var packed = vtable.New();
        var readerHandle = (int)(packed & 0xFFFFFFFF);
        var writerHandle = (int)(packed >> 32);

        return (new FutureReader(readerHandle, vtable), new FutureWriter(writerHandle, vtable));
    }

    public static (FutureReader<T>, FutureWriter<T>) RawFutureNew<T>(FutureVTable vtable)
    {
        var packed = vtable.New();
        var readerHandle = (int)(packed & 0xFFFFFFFF);
        var writerHandle = (int)(packed >> 32);

        return (new FutureReader<T>(readerHandle, vtable), new FutureWriter<T>(writerHandle, vtable));
    }

    public static Task TaskFromStatus(uint status)
    {
        var subTaskStatus = new SubtaskStatus(status);
        status = status & 0xF;

        if(subTaskStatus.IsSubtaskStarting || subTaskStatus.IsSubtaskStarted)
        {
            if(state.WaitableSet == null) {
                state.WaitableSet = AsyncSupport.WaitableSetNew();
            }

            // TODO join and complete the task somwhere.
            TaskCompletionSource tcs = new TaskCompletionSource();
            return tcs.Task;
            // waitableJoin(subtask, state.waitableSet)
            // channel := make(chan uint32)
            // state.pending[subtask] = channel
            // (<-channel)
        }
        else if (subTaskStatus.IsSubtaskReturned)
        {
            return Task.CompletedTask;
        }
        else 
        {
            throw new Exception($"unexpected subtask status: {status}");
        }
    }

    public static Task<T> TaskFromStatus<T>(uint status, Func<T> liftFunc)
    {
        var subTaskStatus = new SubtaskStatus(status);
        status = status & 0xF;

        // TODO join and complete the task somwhere.
        var tcs = new TaskCompletionSource<T>();
        if(subTaskStatus.IsSubtaskStarting || subTaskStatus.IsSubtaskStarted)
        {
            if(state.WaitableSet == null) {
                state.WaitableSet = AsyncSupport.WaitableSetNew();
            }

            return tcs.Task;
        }
        else if (subTaskStatus.IsSubtaskReturned)
        {
            tcs.SetResult(liftFunc());
            return tcs.Task;
        }
        else 
        {
            throw new Exception($"unexpected subtask status: {status}");
        }
    }
}

public class FutureAwaiter : INotifyCompletion {
    public bool IsCompleted => false;
    private FutureReader futureReader;

    public FutureAwaiter(FutureReader futureReader)
    {
        this.futureReader = futureReader;
    }

    public void OnCompleted(Action continuation) 
    {
        var readTask = futureReader.Read();
        
        if(readTask.IsCompleted && !readTask.IsFaulted)
        {
            continuation();
        }
        else
        {
            readTask.ContinueWith(task => 
            {
                if(task.IsFaulted)
                {
                    throw task.Exception!;
                }
                continuation();
            });
        }
    }

    public string GetResult()
    {
        return null;
    }
}

public class FutureReader : IDisposable // : TODO Waitable
{
    FutureAwaiter futureAwaiter;

    public FutureReader(int handle, FutureVTable vTable)
    {
        Handle = handle;
        VTable = vTable;
        futureAwaiter = new FutureAwaiter(this);
    }

    public int Handle { get; private set; }
    public FutureVTable VTable { get; private set; }

    public FutureAwaiter GetAwaiter() => futureAwaiter;

    public int TakeHandle()
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }
        var handle = Handle;
        Handle = 0;
        return handle;
    }

    // TODO: Generate per type for this instrinsic.
    public unsafe Task Read()
    {
        // TODO: Generate for the interop name and the namespace.
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        var status = new WaitableStatus(VTable.StartRead(Handle, IntPtr.Zero));
        if (status.IsBlocked)
        {
            var tcs = new TaskCompletionSource();


            AsyncSupport.ContextTask* contextTaskPtr = (AsyncSupport.ContextTask*)Marshal.AllocHGlobal(sizeof(AsyncSupport.ContextTask));

            AsyncSupport.ContextSet(contextTaskPtr);
            AsyncSupport.PendingCallbacks.TryAdd((IntPtr)contextTaskPtr, tcs);
            return tcs.Task;
        }
        if (status.IsCompleted)
        {
            return Task.CompletedTask;
        }

        throw new NotImplementedException(status.State.ToString());
    }

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        if (Handle != 0)
        {
            VTable.DropReader(Handle);
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~FutureReader()
    {
        Dispose(false);
    }
}

public class FutureReader<T>(int handle, FutureVTable vTable) : IDisposable // : TODO Waitable
{
    public int Handle { get; private set; } = handle;
    public FutureVTable VTable { get; private set; } = vTable;

    public int TakeHandle()
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }
        var handle = Handle;
        Handle = 0;
        return handle;
    }

    // TODO: Generate per type for this instrinsic.
    public unsafe Task Read()
    {
        // TODO: Generate for the interop name and the namespace.
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        var status = new WaitableStatus(vTable.StartRead(Handle, IntPtr.Zero));
        if (status.IsBlocked)
        {
            //TODO: store somewhere so we can complete it later.
            var tcs = new TaskCompletionSource();
            //TODO: Free in callback?
            AsyncSupport.ContextTask* contextTaskPtr = (AsyncSupport.ContextTask*)Marshal.AllocHGlobal(sizeof(AsyncSupport.ContextTask));

            AsyncSupport.ContextSet(contextTaskPtr);
            AsyncSupport.PendingCallbacks.TryAdd((IntPtr)contextTaskPtr, tcs);
            return tcs.Task;
        }
        if (status.IsCompleted)
        {
            return Task.CompletedTask;
        }

        throw new NotImplementedException();
    }

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        if (Handle != 0)
        {
            vTable.DropReader(Handle);
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~FutureReader()
    {
        Dispose(false);
    }
}

/**
 * Helpers for future writer support.
 */
public class FutureWriter(int handle, FutureVTable vTable) // : TODO Waitable
{
    public int Handle { get; } = handle;
    public FutureVTable VTable { get; private set; } = vTable;

    // TODO: Generate per type for this instrinsic.
    public Task Write()
    {
        // TODO: Generate for the interop name.
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        var status = new WaitableStatus(VTable.Write(Handle, IntPtr.Zero));
        if (status.IsBlocked)
        {
            //TODO: store somewhere so we can complete it later.
            var tcs = new TaskCompletionSource();
            return tcs.Task;
        }

        return Task.CompletedTask;
    }

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        if (Handle != 0)
        {
            VTable.DropWriter(Handle);
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~FutureWriter()
    {
        Dispose(false);
    }
}

public class FutureWriter<T>(int handle, FutureVTable vTable) // : TODO Waitable
{
    public int Handle { get; } = handle;
    public FutureVTable VTable { get; private set; } = vTable;

    // TODO: Generate per type for this instrinsic.
    public Task Write()
    {
        // TODO: Generate for the interop name.
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        var status = new WaitableStatus(VTable.Write(Handle, IntPtr.Zero));
        if (status.IsBlocked)
        {
            //TODO: store somewhere so we can complete it later.
            var tcs = new TaskCompletionSource();
            return tcs.Task;
        }

        throw new NotImplementedException();
    }

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        if (Handle != 0)
        {
            VTable.DropWriter(Handle);
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~FutureWriter()
    {
        Dispose(false);
    }
}
