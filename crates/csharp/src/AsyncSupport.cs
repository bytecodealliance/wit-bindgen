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

public enum CallbackCode : int
{
    Exit = 0,
    Yield = 1,
    Wait = 2,
    // TODO:
    //#define TEST_CALLBACK_CODE_WAIT(set) (2 | (set << 4))
}

public enum CancelCode : uint
{
    Completed = 0,
    Dropped = 1,
    Cancelled = 2,
}

// The context that we will create in unmanaged memory and pass to context_set.
// TODO: C has world specific types for these pointers, perhaps C# would benefit from those also.
[StructLayout(LayoutKind.Sequential)]
public struct ContextTask
{
    public int WaitableSetHandle;
    public int FutureHandle;
}

public static class AsyncSupport
{
    private static ConcurrentDictionary<int, ConcurrentDictionary<int, WaitableInfoState>> pendingTasks = new ();
    internal static class PollWasmInterop
    {
        [DllImport("wasi:io/poll@0.2.0", EntryPoint = "poll"), WasmImportLinkage]
        internal static extern void wasmImportPoll(nint p0, int p1, nint p2);
    }

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
        internal static extern void WaitableSetDrop(int waitable);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[context-set-0]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern void ContextSet(ContextTask* waitable);

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[context-get-0]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern ContextTask* ContextGet();

        [global::System.Runtime.InteropServices.DllImport("$root", EntryPoint = "[subtask-drop]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern void SubtaskDrop(int handle);
    }

    public static int WaitableSetNew() 
    {
        return Interop.WaitableSetNew();
    }

    // unsafe because we are using pointers.
    public static unsafe void WaitableSetPoll(int waitableHandle) 
    {
        var error  = Interop.WaitableSetPoll(waitableHandle, null);
        if(error != 0)
        {
            throw new Exception($"WaitableSetPoll failed with error code: {error}");
        }
    }

    internal static void Join(int readerWriterHandle, int waitableHandle, WaitableInfoState waitableInfoState) 
    {
        AddTaskToWaitables(waitableHandle, readerWriterHandle, waitableInfoState);
        Interop.WaitableJoin(readerWriterHandle, waitableHandle);
    }

    // TODO: Revisit this to see if we can remove it.
    // Only allow joining to a handle directly when there is no waitable.
    public static void Join(int handle) 
    {
        Interop.WaitableJoin(handle, 0);
    }

    private static void AddTaskToWaitables(int waitableSetHandle, int waitableHandle, WaitableInfoState waitableInfoState)
    {
        var waitableSetOfTasks = pendingTasks.GetOrAdd(waitableSetHandle, _ => new ConcurrentDictionary<int, WaitableInfoState>());
        waitableSetOfTasks[waitableHandle] = waitableInfoState;
    }

    // unsafe because we use a fixed size buffer.
    public static unsafe EventWaitable WaitableSetWait(int waitableSetHandle) 
    {
        uint* buffer = stackalloc uint[2];
        var eventCode = (EventCode)Interop.WaitableSetWait(waitableSetHandle, buffer);
        Console.WriteLine("WaitableSetWait creating EventWaitable with eventCode " + eventCode);
        return new EventWaitable(eventCode, buffer[0], buffer[1]);
    }

    public static void WaitableSetDrop(int handle) 
    {
        Interop.WaitableSetDrop(handle);
    }

    // unsafe because we are using pointers.
    public static unsafe void ContextSet(ContextTask* contextTask)
    {
        Interop.ContextSet(contextTask);
    }

    // unsafe because we are using pointers.
    public static unsafe ContextTask* ContextGet()
    {
        return Interop.ContextGet();
    }

    // unsafe because we are using pointers.
    public static unsafe int Callback(EventWaitable e, ContextTask* contextPtr)
    {
        Console.WriteLine("Callback");
        ContextTask* contextTaskPtr = ContextGet();

        var waitables = pendingTasks[contextTaskPtr->WaitableSetHandle];
        var waitableInfoState = waitables[e.Waitable];

        if (e.IsDropped)
        {
        Console.WriteLine("Callback e is dropped");
            waitableInfoState.FutureStream!.OtherSideDropped();
        }

        if (e.IsCompleted || e.IsDropped)
        {
        Console.WriteLine("Callback e is completed or dropped");
            // The operation is complete so we can free the buffer and remove the waitable from our dicitonary
            waitables.Remove(e.Waitable, out _);
            if (e.IsSubtask)
            {
                if (e.SubtaskStatus.IsStarting)
                {
                    throw new Exception("unexpected subtask status Starting " + e.Code);
                }
                if (e.SubtaskStatus.IsStarted || e.SubtaskStatus.IsReturned)
                {
                    Console.WriteLine("Callback for subtask started/returned Returned " + e.SubtaskStatus.IsReturned);
                    Console.WriteLine("waitableInfoState is for func " + (waitableInfoState.ForAsyncFunction) );
                    Console.WriteLine("waitableInfoState count " + (e.WaitableCount) );

                    waitableInfoState.SetResult(e.WaitableCount);
                    Interop.SubtaskDrop(e.Waitable);
                }
                else
                {
                    throw new Exception("TODO: subtask status " + e.Code);
                }
            }
            else
            {
                if (e.IsDropped)
                {
                    waitableInfoState.SetException(new StreamDroppedException());
                }
                else
                {
                Console.WriteLine("Callback !subtask and not dropped, setting result, count " + e.WaitableCount);
                    // This may add a new waitable to the set.
                    waitableInfoState.SetResult(e.WaitableCount);
                }
            }

            if (waitables.Count == 0)
            {
                ContextSet(null);
                Marshal.FreeHGlobal((IntPtr)contextTaskPtr);
                return (int)CallbackCode.Exit;
            }

            return (int)CallbackCode.Wait | (int)(contextTaskPtr->WaitableSetHandle << 4);
        }

        throw new NotImplementedException($"WaitableStatus not implemented {e.WaitableStatus.State} in set {contextTaskPtr->WaitableSetHandle}");
    }

    // unsafe because we are using pointers.
    public static unsafe int Callback<T>(EventWaitable e, Func<T> liftFunc)
    {
        Console.WriteLine("Callback");
        ContextTask* contextTaskPtr = ContextGet();

        var waitables = pendingTasks[contextTaskPtr->WaitableSetHandle];
        var waitableInfoState = waitables[e.Waitable];

        if (e.IsDropped)
        {
        Console.WriteLine("Callback e is dropped");
            waitableInfoState.FutureStream!.OtherSideDropped();
        }

        if (e.IsCompleted || e.IsDropped)
        {
        Console.WriteLine("Callback e is completed or dropped");
            // The operation is complete so we can free the buffer and remove the waitable from our dicitonary
            waitables.Remove(e.Waitable, out _);
            if (e.IsSubtask)
            {
                if (e.SubtaskStatus.IsStarting)
                {
                    throw new Exception("unexpected subtask status Starting " + e.Code);
                }
                if (e.SubtaskStatus.IsStarted || e.SubtaskStatus.IsReturned)
                {
                    Console.WriteLine("Callback for subtask started/returned Returned " + e.SubtaskStatus.IsReturned);
                    Console.WriteLine("waitableInfoState is for func " + (waitableInfoState.ForAsyncFunction) );
//                    TaskFromStatus(e.Code, liftFunc);

                    // waitableJoin(event1, 0)
                    Interop.SubtaskDrop(e.Waitable);

                    // Think this is go routine stuff we don't need??
                    // channel := state.pending[event1]
                    // delete(state.pending, event1)
                    // channel <- event2
                }
                else
                {
                    throw new Exception("TODO: subtask status " + e.Code);
                }
            }
            else
            {
                if (e.IsDropped)
                {
                    waitableInfoState.SetException(new StreamDroppedException());
                }
                else
                {
                Console.WriteLine("Callback !subtask and not dropped, setting result, count " + e.WaitableCount);
                    // This may add a new waitable to the set.
                    waitableInfoState.SetResult(e.WaitableCount);
                }
            }

            if (waitables.Count == 0)
            {
                ContextSet(null);
                Marshal.FreeHGlobal((IntPtr)contextTaskPtr);
                return (int)CallbackCode.Exit;
            }

            return (int)CallbackCode.Wait | (int)(contextTaskPtr->WaitableSetHandle << 4);
        }

        throw new NotImplementedException($"WaitableStatus not implemented {e.WaitableStatus.State} in set {contextTaskPtr->WaitableSetHandle}");
    }

    // This method is unsafe because we are using unmanaged memory to store the context.
    internal static unsafe Task TaskFromStatus(uint status)
    {
        var subtaskStatus = new SubtaskStatus(status);
        status = status & 0xF;

        var tcs = new TaskCompletionSource<int>();
        tcs.Task.ContinueWith(t =>
        {
           Console.WriteLine("TaskFromStatus  2 tcs continuewith"); 
        });
        if (subtaskStatus.IsStarting || subtaskStatus.IsStarted)
        {
            ContextTask* contextTaskPtr = ContextGet();
            if (contextTaskPtr == null)
            {
                contextTaskPtr = AllocateAndSetNewContext();
            }

            Join(subtaskStatus.Handle, contextTaskPtr->WaitableSetHandle, new WaitableInfoState(tcs));

            tcs.Task.ContinueWith(t =>
            {
               Console.WriteLine("tcs complete"); 
            });
            return tcs.Task;
        }
        else if (subtaskStatus.IsReturned)
        {
            Console.WriteLine("TaskFromStatus creating completed tcs");
            tcs.SetResult(0);
            return tcs.Task;
        }
        else
        {
            Console.WriteLine("TaskFromStatus unexpected status " + status);
            throw new Exception($"unexpected subtask status: {status}");
        }
    }

    // unsafe because we are using pointers.
    public static unsafe Task<T> TaskFromStatus<T>(uint status, Func<T> liftFunc)
    {
        var subtaskStatus = new SubtaskStatus(status);

        if (subtaskStatus.IsStarting || subtaskStatus.IsStarted)
        {
            ContextTask* contextTaskPtr = ContextGet();
            if (contextTaskPtr == null) {
                contextTaskPtr = AllocateAndSetNewContext();
            }

            var intTaskCompletionSource = new TaskCompletionSource<int>();
            intTaskCompletionSource.Task.ContinueWith( t =>
            {
               Console.WriteLine("intTaskCompletionSource continuewith"); 
            }, TaskContinuationOptions.ExecuteSynchronously);
            var tcs = new LiftingTaskCompletionSource<T>(intTaskCompletionSource, liftFunc);
            Join(subtaskStatus.Handle, contextTaskPtr->WaitableSetHandle, new WaitableInfoState(intTaskCompletionSource));

            tcs.Task.ContinueWith(t =>
            {
               Console.WriteLine("lifting tcs complete"); 
            }, TaskContinuationOptions.ExecuteSynchronously);
            return tcs.Task;
        }
        else if (subtaskStatus.IsReturned)
        {
            var tcs = new TaskCompletionSource<T>();
            Console.WriteLine("creating completed tcs");
            tcs.SetResult(liftFunc());
            return tcs.Task;
        }
        else 
        {
            throw new Exception($"unexpected subtask status: {status}");
        }
    }

    // Placeholder, TODO: Needs implementing for async functions that return values.
    internal class LiftingTaskCompletionSource<T> : TaskCompletionSource<T>
    {
        internal LiftingTaskCompletionSource(TaskCompletionSource<int> innerTaskCompletionSource, Func<T> _liftFunc)
        {
            innerTaskCompletionSource.Task.ContinueWith(t => {
                Console.WriteLine("LiftingTaskCompletionSource inner task completed with status " + t.Status);
                if (t.Status == TaskStatus.RanToCompletion)
                {
                    try
                    {
                        var liftedResult = _liftFunc();
                        Console.WriteLine("Lifted result " + liftedResult);
                        SetResult(liftedResult);
                    }
                    catch(Exception e)
                    {
                        Console.WriteLine("Exception in lift function: " + e);
                        SetException(e);
                    }
                }
                else if (t.Status == TaskStatus.Faulted)
                {
                    Console.WriteLine("Inner task faulted with exception " + t.Exception);
                    SetException(t.Exception!);
                }
                else if (t.Status == TaskStatus.Canceled)
                {
                    Console.WriteLine("Inner task was canceled");
                    SetCanceled();
                }
                throw new NotImplementedException("LiftingTaskCompletionSource unexpected task status " + t.Status);
            }, TaskContinuationOptions.ExecuteSynchronously);
        }
    }

    // unsafe because we are working with native memory.
    internal static unsafe ContextTask* AllocateAndSetNewContext()
    {
        var contextTaskPtr = (ContextTask *)Marshal.AllocHGlobal(Marshal.SizeOf<ContextTask>());
        contextTaskPtr->WaitableSetHandle = WaitableSetNew();
        ContextSet(contextTaskPtr);
        return contextTaskPtr;
    }
}


/**
 * Future/Stream VTable delegate types.
 */
public delegate ulong New();
public delegate uint FutureRead(int handle, IntPtr buffer);
public delegate void DropReader(int handle);
public delegate void DropWriter(int handle);
public delegate uint FutureWrite(int handle, IntPtr buffer);

public delegate uint StreamWrite(int handle, IntPtr buffer, uint length);
public delegate uint StreamRead(int handle, IntPtr buffer, uint length);
public delegate Array Lift(IntPtr buffer, Array? resultBuffer);
public delegate void Lower(object payload);
public delegate uint CancelRead(int handle);
public delegate uint CancelWrite(int handle);

public interface ICancelableWrite
{
    uint CancelWrite(int handle);
}

public interface ICancelable
{
    uint Cancel();
}

public class CancelableRead(IVTable cancelableVTable, int handle) : ICancelable
{
    public uint Cancel()
    {
        return cancelableVTable.CancelRead(handle);        
    }
}

public class CancelableWrite(ICancelableWrite cancelableVTable, int handle) : ICancelable
{
    public uint Cancel()
    {
        return cancelableVTable.CancelWrite(handle);        
    }
}

/// <summary>
/// Common to all VTables.  TODO: Delete ICancelableWrite?
/// </summary>
public interface IVTable
{
    uint CancelRead(int handle);
    uint Size { get; set; }
    uint Align { get; set; }
}

public struct FutureVTable : ICancelableWrite, IVTable
{
    // Generated code even if we are not using futures, so disable the warning.
#pragma warning disable 649
    internal New New;
    internal FutureRead Read;
    internal FutureWrite Write;
    internal DropReader DropReader;
    internal DropWriter DropWriter;
    internal Lift? Lift;
    internal Lower? Lower;
    internal CancelWrite CancelWriteDelegate;
    internal CancelRead CancelReadDelegate;
#pragma warning disable 649

    // The size and alignment of the buffer.
    public uint Size { get; set; }
    public uint Align { get; set; }
    public uint CancelRead(int handle)
    {
        return CancelReadDelegate(handle);
    }

    public uint CancelWrite(int handle)
    {
        return CancelWriteDelegate(handle);
    }
}

public struct StreamVTable : ICancelableWrite, IVTable
{
    // Generated code even if we are not using streams, so disable the warning.
#pragma warning disable 649
    internal New New;
    internal StreamRead Read;
    internal StreamWrite Write;
    internal DropReader DropReader;
    internal DropWriter DropWriter;
    internal Lift? Lift;
    internal Lower? Lower;
    internal CancelWrite CancelWriteDelegate;
    internal CancelRead CancelReadDelegate;
#pragma warning disable 649

    // The size and alignment of the buffer.
    public uint Size { get; set; }
    public uint Align { get; set; }

    public uint CancelRead(int handle)
    {
        return CancelReadDelegate(handle);
    }

    public uint CancelWrite(int handle)
    {
        return CancelWriteDelegate(handle);
    }
}

internal interface IFutureStream : IDisposable
{
    void FreeBuffer();
    // Called when notified the other side is dropped.
    void OtherSideDropped();
}

public static class FutureHelpers
{
    /// Helper function to create a new read/write pair for a component model
    /// future.
    internal static (FutureReader, FutureWriter) RawFutureNew(FutureVTable vtable)
    {
        var packed = vtable.New();
        var readerHandle = (int)(packed & 0xFFFFFFFF);
        var writerHandle = (int)(packed >> 32);

        return (new FutureReader(readerHandle, vtable), new FutureWriter(writerHandle, vtable));
    }

    internal static (FutureReader<T>, FutureWriter<T>) RawFutureNew<T>(FutureVTable vtable)
    {
        var packed = vtable.New();
        var readerHandle = (int)(packed & 0xFFFFFFFF);
        var writerHandle = (int)(packed >> 32);

        return (new FutureReader<T>(readerHandle, vtable), new FutureWriter<T>(writerHandle, vtable));
    }

    /// Helper function to create a new read/write pair for a component model
    /// stream.
    internal static (StreamReader, StreamWriter) RawStreamNew(StreamVTable vtable)
    {
        var packed = vtable.New();
        var readerHandle = (int)(packed & 0xFFFFFFFF);
        var writerHandle = (int)(packed >> 32);

        return (new StreamReader(readerHandle, vtable), new StreamWriter(writerHandle, vtable));
    }

    internal static (StreamReader<T>, StreamWriter<T>) RawStreamNew<T>(StreamVTable vtable)
    {
        var packed = vtable.New();
        var readerHandle = (int)(packed & 0xFFFFFFFF);
        var writerHandle = (int)(packed >> 32);

        return (new StreamReader<T>(readerHandle, vtable), new StreamWriter<T>(writerHandle, vtable));
    }
}

public abstract class ReaderBase : IFutureStream
{
    private bool writerDropped;

    internal ReaderBase(int handle)
    {
        Handle = handle;
    }

    internal int Handle { get; private set; }

    internal int TakeHandle()
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }
        var handle = Handle;
        Handle = 0;
        return handle;
    }

    protected unsafe IntPtr GetBuffer<T>(int length, T[]? userBuffer, IVTable vTable, List<Action> cleanups)
    {
        // For primitive, blittable types, TODO: this probably does not align 100% with the component ABI?
        if (typeof(T).IsPrimitive || typeof(T).IsValueType)
        {
            T[] buffer;
            // For Streams, the user passes the buffer, so use that.
            if(userBuffer != null)
            {
                buffer = userBuffer;
            }
            else
            {
                buffer = new T[length];
            }
            var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
            cleanups.Add(() => handle.Free());
            return handle.AddrOfPinnedObject();
        }
        else
        {
            System.Diagnostics.Debug.Assert(vTable.Size > 0, $"Did not compute size for {typeof(T)}.");
            IntPtr bufferPtr = (IntPtr)global::System.Runtime.InteropServices.NativeMemory.AlignedAlloc(vTable.Size, vTable.Align);
            cleanups.Add(() => global::System.Runtime.InteropServices.NativeMemory.Free((void*)bufferPtr));
            Console.WriteLine("creating buffer of type " + typeof(T).FullName);
            return bufferPtr;
        }
    }

    protected unsafe T[] LiftBuffer<T>(IntPtr buffer, T[] resultBuffer, Lift? liftFunc)
    {
        // For primitive, blittable types
        if (typeof(T).IsPrimitive || typeof(T).IsValueType)
        {
            // TODO array length > 1
            resultBuffer[0] = *(T*)buffer;
        }
        else
        {
            Console.WriteLine("Lifting buffer of type " + typeof(T).FullName + " is not supported without a custom lower function.");
            liftFunc(buffer, resultBuffer);
        }

        return resultBuffer;
    }

    internal abstract uint VTableRead(IntPtr bufferPtr, int length);

    // unsafe as we are working with pointers.
    internal unsafe ComponentTask<int> ReadInternal(IntPtr buffer, int length, IVTable vtable)
    {
        Console.WriteLine("ReadInternal start");
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }
        Console.WriteLine("ReadInternal Handle is valid");

        if (writerDropped)
        {
            throw new StreamDroppedException();    
        }
        Console.WriteLine("ReadInternal writer is not dropped");

        Console.WriteLine("Reading");
        var status = new WaitableStatus(VTableRead(buffer, length));
        if (status.IsBlocked)
        {
        Console.WriteLine("Blocked");
            var task = new ComponentTask<int>(new CancelableRead(vtable, Handle));
            ContextTask* contextTaskPtr = AsyncSupport.ContextGet();
            if(contextTaskPtr == null)
            {
                contextTaskPtr = AsyncSupport.AllocateAndSetNewContext();
            }

            AsyncSupport.Join(Handle, contextTaskPtr->WaitableSetHandle, new WaitableInfoState(task, this));
            return task;
        }
        if (status.IsCompleted)
        {
        Console.WriteLine("complete");

            return ComponentTask<int>.FromResult((int)status.Count);
        }

        throw new NotImplementedException(status.State.ToString());
    }

    void IFutureStream.FreeBuffer()
    {
        // TODO:dealloc lists here?
    }

    void IFutureStream.OtherSideDropped()
    {
        writerDropped = true;
    }

    internal abstract void VTableDrop();

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        if (Handle != 0)
        {
            VTableDrop();
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~ReaderBase()
    {
        Dispose(false);
    }
}

public class FutureReader : ReaderBase
{
    internal FutureReader(int handle, FutureVTable vTable) : base(handle)
    {
        VTable = vTable;
    }

    internal FutureVTable VTable { get; private set; }

    public ComponentTask Read()
    {
        return ReadInternal(IntPtr.Zero, 0, VTable);
    }

    // TODO: delete this method?
    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr);
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }
}

public class FutureReader<T>(int handle, FutureVTable vTable) : ReaderBase(handle)
{
    public FutureVTable VTable { get; private set; } = vTable;

    public ComponentTask<T> Read()
    {
        Console.WriteLine("FutureReader<T> reading");
        var cleanups = new List<Action>();
        var buf = GetBuffer<T>(1, null /* We need the buffer created for us */, VTable, cleanups);
        ComponentTask<int> internalTask = ReadInternal(buf, 1, VTable);

        // Wrap the task so we can return a T and not the number of Ts read
        ComponentTask<T> readTask = new(new DelegatingCancelable(internalTask));

        internalTask.ContinueWith(it =>
        {
        Console.WriteLine("FutureReader<T> reading internalTask ContinueWith");
            if (it.IsCompletedSuccessfully)
            {
        Console.WriteLine("FutureReader<T> reading internalTask SetResult");
                try
                {
                readTask.SetResult(((T[])VTable.Lift(buf, new T[1]))[0]);
                    
                }
                catch(Exception e)
                {
        Console.WriteLine("FutureReader<T> reading internalTask SetResult exception " + e);
                    
                }
        Console.WriteLine("FutureReader<T> reading internalTask SetResult complete");
            }
            else if (!it.IsCanceled)
            {
                //TODO
                throw new NotImplementedException("faulted future read not implemented");
            }

            foreach(var cleanup in cleanups)
            {
                cleanup();
            }
        });
        return readTask;
    }

    class DelegatingCancelable : ICancelable
    {
        private ComponentTask innerTask;

        internal DelegatingCancelable(ComponentTask innerTask)
        {
            this.innerTask = innerTask;
        }

        uint ICancelable.Cancel()
        {
            var cancelVal = innerTask.Cancel();
            return (uint)cancelVal;
        }
    }

    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr);
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }
}

public class StreamReader : ReaderBase
{
    public StreamReader(int handle, StreamVTable vTable) : base(handle)
    {
        VTable = vTable;
    }

    public StreamVTable VTable { get; private set; }

    public ComponentTask Read(int length)
    {
        return ReadInternal(IntPtr.Zero, length, VTable);
    }

    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr, (uint)length);
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }
}

public class StreamReader<T>(int handle, StreamVTable vTable) :  ReaderBase(handle)
{
    public StreamVTable VTable { get; private set; } = vTable;

    public ComponentTask<int> Read(T[] resultBuffer)
    {
        var cleanups = new List<Action>();
        var buf = GetBuffer<T>(resultBuffer.Length, resultBuffer, VTable, cleanups);

        var task = ReadInternal(buf, resultBuffer.Length, VTable);
        task.ContinueWith(it =>
        {
            if (it.IsCompletedSuccessfully)
            {
                VTable.Lift(buf, resultBuffer);
            }
            else if (!it.IsCanceled)
            {
                //TODO
                throw new NotImplementedException("faulted stream read not implemented");
            }

            foreach(var cleanup in cleanups)
            {
                cleanup();
            }
        });

        return task;
    }

    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr, (uint)length);
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }
}

public abstract class WriterBase : IFutureStream
{
    private nint bufferPtr;
    private bool readerDropped;
    private bool canDrop;

    internal WriterBase(int handle)
    {
        Handle = handle;
    }

    internal int Handle { get; private set; }

    internal int TakeHandle()
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }
        var handle = Handle;
        Handle = 0;
        return handle;
    }

    internal abstract uint VTableWrite(IntPtr bufferPtr, int length);

    // unsafe as we are working with pointers.
    internal unsafe ComponentTask<int> WriteInternal(Func<nint> lowerPayload, int length, ICancelableWrite cancelable)
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        if (readerDropped)
        {
            throw new StreamDroppedException();    
        }
        bufferPtr = lowerPayload();

        var status = new WaitableStatus(VTableWrite(bufferPtr, length));
        canDrop = true;  // We can only call drop once something has been written.
        if (status.IsBlocked)
        {
            Console.WriteLine("Write blocked for length " + length);
            var tcs = new ComponentTask<int>(new CancelableWrite(cancelable, Handle));
            tcs.ContinueWith(t =>
            {
                if (t.IsCanceled)
                {
                    canDrop = false;
                }
            });

            ContextTask* contextTaskPtr = AsyncSupport.ContextGet();
            if(contextTaskPtr == null)
            {
                contextTaskPtr = AsyncSupport.AllocateAndSetNewContext();
            }
            AsyncSupport.Join(Handle, contextTaskPtr->WaitableSetHandle, new WaitableInfoState(tcs, this));
            return tcs;
        }

        if (status.IsCompleted)
        {
            return ComponentTask<int>.FromResult((int)status.Count);
        }

        throw new NotImplementedException($"Unsupported write status {status.State}");
    }

    void IFutureStream.FreeBuffer()
    {
        // TODO: Is this the responsiblity of the writer or the reader?
        Console.WriteLine("Warning write buffer free not implemented, leaks.");
    }

    void IFutureStream.OtherSideDropped()
    {
        readerDropped = true;
    }

    internal abstract void VTableDrop();

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        if (Handle != 0 && canDrop)
        {
            VTableDrop();
        }
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~WriterBase()
    {
        Dispose(false);
    }
}

public class FutureWriter(int handle, FutureVTable vTable) : WriterBase(handle)
{
    public FutureVTable VTable { get; private set; } = vTable;

    public ComponentTask<int> Write()
    {
        return WriteInternal(() => 0, 0, VTable);
    }

    internal override uint VTableWrite(IntPtr bufferPtr, int length)
    {
        return VTable.Write(Handle, bufferPtr);
    }

    internal override void VTableDrop()
    {
        VTable.DropWriter(Handle);
    }
}

public class FutureWriter<T>(int handle, FutureVTable vTable) : WriterBase(handle)
{
    public FutureVTable VTable { get; private set; } = vTable;

    private nint LowerPayload(T payload)
    {
        if (VTable.Lower == null)
        {
            return GCHandle.Alloc(payload, GCHandleType.Pinned).AddrOfPinnedObject();
        }
        else
        {
            // Lower the payload
            VTable.Lower(payload);
            return InteropReturnArea.returnArea.AddressOfReturnArea();
        }
    }

    public ComponentTask<int> Write(T payload)
    {
        return WriteInternal(() => LowerPayload(payload), 1, VTable);
    }

    internal override uint VTableWrite(IntPtr bufferPtr, int length)
    {
        return VTable.Write(Handle, bufferPtr);
    }

    internal override void VTableDrop()
    {
        VTable.DropWriter(Handle);
    }
}

public class StreamWriter(int handle, StreamVTable vTable) : WriterBase(handle)
{
    public StreamVTable VTable { get; private set; } = vTable;

    public ComponentTask<int> Write()
    {
        return WriteInternal(() => 0, 0, VTable);
    }

    internal override uint VTableWrite(IntPtr bufferPtr, int length)
    {
        return VTable.Write(Handle, bufferPtr, (uint)length);
    }

    internal override void VTableDrop()
    {
        VTable.DropWriter(Handle);
    }
}

public class StreamWriter<T>(int handle, StreamVTable vTable) : WriterBase(handle)
{
    private nint bufferPtr;
    public StreamVTable VTable { get; private set; } = vTable;

    private nint LowerPayload(T[] payload)
    {
        if (VTable.Lower == null)
        {
            return GCHandle.Alloc(payload, GCHandleType.Pinned).AddrOfPinnedObject();
        }
        else
        {
            // Lower the payload
            VTable.Lower(payload);
            return InteropReturnArea.returnArea.AddressOfReturnArea();
        }
    }

    public ComponentTask<int> Write(T[] payload)
    {
        return WriteInternal(() => LowerPayload(payload), payload.Length, VTable);
    }

    internal override uint VTableWrite(IntPtr bufferPtr, int length)
    {
        return VTable.Write(Handle, bufferPtr, (uint)length);
    }

    internal override void VTableDrop()
    {
        VTable.DropWriter(Handle);
    }
}

internal struct WaitableInfoState
{
    internal WaitableInfoState(ComponentTask<int> componentTaskInt, IFutureStream futureStream)
    {
        this.componentTaskInt = componentTaskInt;
        FutureStream = futureStream;        
    }

    internal WaitableInfoState(TaskCompletionSource<int> taskCompletionSource)
    {
        this.taskCompletionSource = taskCompletionSource;
        try
        {
            throw new Exception("Creating WaitableInfoState with TaskCompletionSource, stack trace: " + Environment.StackTrace);
        }
        catch (Exception e)
        {
            Console.WriteLine("Exception in WaitableInfoState constructor: " + e);
        }
    }

    internal void SetResult(int count)
    {
        Console.WriteLine("SetResult");
        if (taskCompletionSource != null)
        {
        Console.WriteLine("SetResult on tcs " + taskCompletionSource.Task.Status + " " + taskCompletionSource.GetType());
        try
        {
            taskCompletionSource.SetResult(count);
        }
        catch(Exception e)
        {
            Console.WriteLine("Exception in SetResult on tcs: " + e);
        }
        }
        else if (componentTask != null)
        {
        Console.WriteLine("SetResult on componentTask");
            componentTask.SetResult();
        }
        else if (componentTaskInt != null)
        {
        Console.WriteLine("SetResult on componentTaskInt");
            componentTaskInt.SetResult(count);
        }
        else
        {
            throw new InvalidOperationException("No component task associated with this WaitableInfoState.");
        }
    }

    internal void SetException(Exception e)
    {
        if (componentTask != null)
        {
            componentTask.SetException(e);
        }
        else
        {
            componentTaskInt.SetException(e);
        }
    }

#if DEBUG
    // For debugging
    public readonly bool ForAsyncFunction => taskCompletionSource != null;
#endif 

    // We have a taskCompletionSource for an async function, a ComponentTask for a future or stream.
    private TaskCompletionSource<int>? taskCompletionSource;
    private ComponentTask? componentTask;
    private ComponentTask<int>? componentTaskInt;
    internal IFutureStream? FutureStream;
}

public class StreamDroppedException : Exception
{
    public StreamDroppedException() : base()
    {
    }

    public StreamDroppedException(string message) : base(message)
    {
    }
}

public abstract class ComponentTask
{
    protected readonly ICancelable cancelableVTable;
    private bool canCancel = true;

    internal ComponentTask(ICancelable? cancelableVTable = null)
    {
        this.cancelableVTable = cancelableVTable;
    }

    public abstract Task Task { get; }

    public abstract bool IsCompleted { get; }

    public CancelCode Cancel()
    {
        if(!canCancel)
        {
            return CancelCode.Completed;
        }

        if(cancelableVTable == null)
        {
            throw new InvalidOperationException("Cannot cancel a task that was created as completed with a result.");
        }

        uint cancelReturn = cancelableVTable.Cancel();
        SetCanceled();
        return (CancelCode)cancelReturn;
    }

    public abstract void SetCanceled();

    public virtual void SetResult()
    {
        canCancel = false;
    }

    public abstract void SetException(Exception e);

    public static ComponentTask FromResult()
    {
        var task = new ComponentTask<int>();
        task.SetResult(0);
        return task;
    }

    /// <summary>
    /// Makes the class directly awaitable.
    /// </summary>
    public TaskAwaiter GetAwaiter()
    {
        return Task.GetAwaiter();
    }
}

public class ComponentTask<T> : ComponentTask
{
    private readonly TaskCompletionSource<T> tcs;

    internal ComponentTask(ICancelable? cancelableVTable = null) : base(cancelableVTable)
    {
        tcs = new TaskCompletionSource<T>();
        tcs.Task.ContinueWith(t =>
        {
            Console.WriteLine("ComponentTask<T> tcs completed with status " + t.Status);
        });
    }

    public override Task Task => tcs.Task;

    public override bool IsCompleted => tcs.Task.IsCompleted;

    public Task ContinueWith(Action<Task<T>> continuationAction)
    {
        return tcs.Task.ContinueWith(continuationAction, TaskContinuationOptions.ExecuteSynchronously);
    }

    public void SetResult(T result) 
    {
        SetResult();
        tcs.SetResult(result);
    }

    public static ComponentTask<T> FromResult(T result)
    {
        var task = new ComponentTask<T>();
        task.tcs.SetResult(result);
        return task;
    }

    public override void SetCanceled()
    {
        tcs.SetCanceled();
    }

    public override void SetException(Exception e)
    {
        tcs.SetException(e);
    }

    /// <summary>
    /// Makes the class directly awaitable.
    /// </summary>
    public new TaskAwaiter<T> GetAwaiter()
    {
        return tcs.Task.GetAwaiter();
    }
            
    public T Result => tcs.Task.Result;
}