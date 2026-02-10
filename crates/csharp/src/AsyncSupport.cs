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
    Wait = 2,
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
    private static ConcurrentDictionary<int, ConcurrentDictionary<int, WaitableInfoState>> pendingTasks = new ();
    internal static class PollWasmInterop
    {
        [DllImport("wasi:io/poll@0.2.0", EntryPoint = "poll"), WasmImportLinkage]
        internal static extern void wasmImportPoll(nint p0, int p1, nint p2);
    }

    // TODO: How do we allow multiple waitable sets?
    internal static WaitableSet WaitableSet;

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
        Console.WriteLine($"WaitableSet created with number {waitableSet}");
        return new WaitableSet(waitableSet);
    }

    public static unsafe void WaitableSetPoll(int waitableHandle) 
    {
        var error  = Interop.WaitableSetPoll(waitableHandle, null);
        if(error != 0)
        {
            throw new Exception($"WaitableSetPoll failed with error code: {error}");
        }
    }

    internal static void Join(SubtaskStatus subtask, WaitableSet set, WaitableInfoState waitableInfoState) 
    {
        AddTaskToWaitables(set.Handle, subtask.Handle, waitableInfoState);
        Interop.WaitableJoin(subtask.Handle, set.Handle);
    }

    internal static void Join(ReaderBase reader, WaitableSet set, WaitableInfoState waitableInfoState) 
    {
        AddTaskToWaitables(set.Handle, reader.Handle, waitableInfoState);
        Interop.WaitableJoin(reader.Handle, set.Handle);
    }
    internal static void Join<T>(FutureReader<T> reader, WaitableSet set, WaitableInfoState waitableInfoState) 
    {
        AddTaskToWaitables(set.Handle, reader.Handle, waitableInfoState);
        Interop.WaitableJoin(reader.Handle, set.Handle);
    }

    internal static void Join(FutureWriter writer, WaitableSet set, WaitableInfoState waitableInfoState) 
    {
        // Store the task completion source so we can complete it later
        AddTaskToWaitables(set.Handle, writer.Handle, waitableInfoState);
        Interop.WaitableJoin(writer.Handle, set.Handle);
    }

    internal static void Join<T>(StreamWriter<T> writer, WaitableSet set, WaitableInfoState waitableInfoState) 
    {
        // Store the task completion source so we can complete it later
        AddTaskToWaitables(set.Handle, writer.Handle, waitableInfoState);
        Interop.WaitableJoin(writer.Handle, set.Handle);
    }

    // TODO: Revisit this to see if we can remove it.
    // Only allow joining to a handle directly when there is no waitable.
    public static void Join(int handle) 
    {
        Interop.WaitableJoin(handle, 0);
    }

    private static void AddTaskToWaitables(int waitableSetHandle, int waitableHandle, WaitableInfoState waitableInfoState)
    {
        Console.WriteLine($"Adding waitable {waitableHandle} to set {waitableSetHandle}");
        var waitableSetOfTasks = pendingTasks.GetOrAdd(waitableSetHandle, _ => new ConcurrentDictionary<int, WaitableInfoState>());
        waitableSetOfTasks[waitableHandle] = waitableInfoState;
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

    public static unsafe uint Callback(EventWaitable e, ContextTask* contextPtr, Action taskReturn)
    {
        Console.WriteLine($"Callback Event code {e.EventCode} Code {e.Code} Waitable {e.Waitable} Waitable Status {e.WaitableStatus.State}, Count {e.WaitableCount}");
        var waitables = pendingTasks[WaitableSet.Handle];
        var waitableInfoState = waitables[e.Waitable];

        if (e.IsDropped)
        {
            Console.WriteLine("Dropped.");
            waitableInfoState.FutureStream.OtherSideDropped();
        }

        if (e.IsCompleted || e.IsDropped)
        {
            // The operation is complete so we can free the buffer and remove the waitable from our dicitonary
            Console.WriteLine("Setting the result");
            waitables.Remove(e.Waitable, out _);
            if (e.IsSubtask)
            {
                // TODO: Handle/lift async function return values. 
                waitableInfoState.SetResult(0 /* not used */);
            }
            else
            {
                waitableInfoState.FutureStream.FreeBuffer();

                if (e.IsDropped)
                {
                    waitableInfoState.SetException(new StreamDroppedException());
                }
                else
                {
                    // This may add a new waitable to the set.
                    waitableInfoState.SetResult(e.WaitableCount);
                }
            }

            if (waitables.Count == 0)
            {
                Console.WriteLine($"No more waitables for waitable {e.Waitable} in set {WaitableSet.Handle}");
                taskReturn();
                return (uint)CallbackCode.Exit;
            }

            Console.WriteLine("More waitables in the set.");
            return (uint)CallbackCode.Wait | (uint)(WaitableSet.Handle << 4);
        }

        throw new NotImplementedException($"WaitableStatus not implemented {e.WaitableStatus.State} in set {WaitableSet.Handle}");
    }

    public static Task TaskFromStatus(uint status)
    {
        var subtaskStatus = new SubtaskStatus(status);
        status = status & 0xF;

        if (subtaskStatus.IsSubtaskStarting || subtaskStatus.IsSubtaskStarted)
        {
            if (WaitableSet == null) {
                WaitableSet = WaitableSetNew();
                Console.WriteLine($"TaskFromStatus creating WaitableSet {WaitableSet.Handle}");
            }

            TaskCompletionSource tcs = new TaskCompletionSource();
            AsyncSupport.Join(subtaskStatus, WaitableSet, new WaitableInfoState(tcs));
            return tcs.Task;
        }
        else if (subtaskStatus.IsSubtaskReturned)
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
        var subtaskStatus = new SubtaskStatus(status);
        status = status & 0xF;

        // TODO join and complete the task somwhere.
        var tcs = new TaskCompletionSource<T>();
        if (subtaskStatus.IsSubtaskStarting || subtaskStatus.IsSubtaskStarted)
        {
            if (WaitableSet == null) {
                Console.WriteLine("TaskFromStatus<T> creating WaitableSet");
                WaitableSet = AsyncSupport.WaitableSetNew();
            }

            return tcs.Task;
        }
        else if (subtaskStatus.IsSubtaskReturned)
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
public delegate void Lower(object payload, nint size);

public struct FutureVTable
{
    public New New;
    public FutureRead Read;
    public FutureWrite Write;
    public DropReader DropReader;
    public DropWriter DropWriter;
}

public struct StreamVTable
{
    public New New;
    public StreamRead Read;
    public StreamWrite Write;
    public DropReader DropReader;
    public DropWriter DropWriter;
    public Lower? Lower;
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

        Console.WriteLine($"Createing reader<T> with handle {readerHandle}");
        Console.WriteLine($"Createing writer<T> with handle {writerHandle}");
        return (new StreamReader<T>(readerHandle, vtable), new StreamWriter<T>(writerHandle, vtable));
    }
}

internal struct WaitableInfoState
{
    internal WaitableInfoState(TaskCompletionSource<int> taskCompletionSource, IFutureStream futureStream)
    {
        taskCompletionSourceInt = taskCompletionSource;
        FutureStream = futureStream;        
    }

    internal WaitableInfoState(TaskCompletionSource taskCompletionSource, IFutureStream futureStream)
    {
        this.taskCompletionSource = taskCompletionSource;
        FutureStream = futureStream;        
    }

    internal WaitableInfoState(TaskCompletionSource taskCompletionSource)
    {
        this.taskCompletionSource = taskCompletionSource;
    }

    internal void SetResult(int count)
    {
        if (taskCompletionSource != null)
        {
            Console.WriteLine("Setting result for void waitable completion source");
            taskCompletionSource.SetResult();
        }
        else
        {
            taskCompletionSourceInt.SetResult(count);
        }
    }

    internal void SetException(Exception e)
    {
        if (taskCompletionSource != null)
        {
            Console.WriteLine("Setting exception waitable completion source");
            taskCompletionSource.SetException(e);
        }
        else
        {
            taskCompletionSourceInt.SetException(e);
        }
    }

    private TaskCompletionSource taskCompletionSource;
    private TaskCompletionSource<int> taskCompletionSourceInt;
    internal IFutureStream FutureStream;
}

public abstract class ReaderBase : IFutureStream
{
    private GCHandle? bufferHandle;
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

    internal abstract uint VTableRead(IntPtr bufferPtr, int length);

    internal unsafe Task<int> ReadInternal(Func<GCHandle?> liftBuffer, int length)
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        if (writerDropped)
        {
            throw new StreamDroppedException();    
        }

        bufferHandle = liftBuffer();
        var status = new WaitableStatus(VTableRead(bufferHandle == null ? IntPtr.Zero : bufferHandle.Value.AddrOfPinnedObject(), length));
        if (status.IsBlocked)
        {
            Console.WriteLine("Read Blocked");
            var tcs = new TaskCompletionSource<int>();
            if(AsyncSupport.WaitableSet == null)
            {
            Console.WriteLine("FutureReader Read Blocked creating WaitableSet");
                AsyncSupport.WaitableSet = AsyncSupport.WaitableSetNew();
            }
            Console.WriteLine("blocked read before join");

            Join(AsyncSupport.WaitableSet, tcs);
            Console.WriteLine("blocked read after join");
            return tcs.Task;
        }
        if (status.IsCompleted)
        {
            return Task.FromResult((int)status.Count);
        }

        throw new NotImplementedException(status.State.ToString());
    }

    internal abstract void Join(WaitableSet waitableSet, TaskCompletionSource<int> tcs);

    void IFutureStream.FreeBuffer()
    {
        bufferHandle?.Free();
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

    public unsafe Task Read()
    {
        return ReadInternal(() => null, 0);
    }

    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr);
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }

    internal override void Join(WaitableSet waitableSet, TaskCompletionSource<int> tcs)
    {
        AsyncSupport.Join(this, waitableSet, new WaitableInfoState(tcs, this));
    }
}

public class FutureReader<T>(int handle, FutureVTable vTable) : ReaderBase(handle)
{
    public FutureVTable VTable { get; private set; } = vTable;

    private GCHandle LiftBuffer<T>(T buffer)
    {
        if(typeof(T) == typeof(byte))
        {
            return GCHandle.Alloc(buffer, GCHandleType.Pinned);
        }
        else
        {
            // TODO: crete buffers for lowered stream types and then lift
            throw new NotImplementedException("reading from futures types that require lifting");
        }
    }

    public unsafe Task Read<T>(T buffer)
    {
        return ReadInternal(() => LiftBuffer(buffer), 1);
    }

    internal override void Join(WaitableSet waitableSet, TaskCompletionSource<int> tcs)
    {
        AsyncSupport.Join(this, waitableSet, new WaitableInfoState(tcs, this));
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

    public unsafe Task Read(int length)
    {
        return ReadInternal(() => null, length);
    }

    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr, (uint)length);
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }

    internal override void Join(WaitableSet waitableSet, TaskCompletionSource<int> tcs)
    {
        AsyncSupport.Join(this, waitableSet, new WaitableInfoState(tcs, this));
    }
}

public class StreamReader<T>(int handle, StreamVTable vTable) :  ReaderBase(handle)
{
    public StreamVTable VTable { get; private set; } = vTable;

    private GCHandle LiftBuffer<T>(T[] buffer)
    {
        if(typeof(T) == typeof(byte))
        {
            return GCHandle.Alloc(buffer, GCHandleType.Pinned);
        }
        else
        {
            // TODO: crete buffers for lowered stream types and then lift
            throw new NotImplementedException("reading from stream types that require lifting");
        }
    }

    public unsafe Task<int> Read<T>(T[] buffer)
    {
        return ReadInternal(() => LiftBuffer(buffer), buffer.Length);
    }

    internal override uint VTableRead(IntPtr ptr, int length)
    {
        return VTable.Read(Handle, ptr, (uint)length);
    }

    internal override void Join(WaitableSet waitableSet, TaskCompletionSource<int> tcs)
    {
        AsyncSupport.Join(this, waitableSet, new WaitableInfoState(tcs, this));
    }

    internal override void VTableDrop()
    {
        VTable.DropReader(Handle);
    }
}

public class FutureWriter(int handle, FutureVTable vTable) : IFutureStream
{
    public int Handle { get; } = handle;
    public FutureVTable VTable { get; private set; } = vTable;
    private bool readerDropped;

    public Task Write()
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        if (readerDropped)
        {
            throw new StreamDroppedException();    
        }

        var status = new WaitableStatus(VTable.Write(Handle, IntPtr.Zero));
        if (status.IsBlocked)
        {
            Console.WriteLine("blocked write");
            var tcs = new TaskCompletionSource();
            if(AsyncSupport.WaitableSet == null)
            {
                AsyncSupport.WaitableSet = AsyncSupport.WaitableSetNew();
            }
            Console.WriteLine("blocked write before join");
            AsyncSupport.Join(this, AsyncSupport.WaitableSet, new WaitableInfoState(tcs, this));
            Console.WriteLine("blocked write after join");
            return tcs.Task;
        }

        if (status.IsCompleted)
        {
            return Task.CompletedTask;
        }

        throw new NotImplementedException($"Unsupported write status {status.State}");
    }

    void IFutureStream.FreeBuffer()
    {
    }

    void IFutureStream.OtherSideDropped()
    {
        readerDropped = true;
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

public class FutureWriter<T>(int handle, FutureVTable vTable) : IFutureStream
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

        throw new NotImplementedException($"Unsupported write status {status.State}");
    }

    void IFutureStream.FreeBuffer()
    {
    }

    void IFutureStream.OtherSideDropped()
    {
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

/**
 * Helpers for stream writer support.
 */
public class StreamWriter(int handle, StreamVTable vTable) : IFutureStream
{
    private GCHandle bufferHandle;
    private bool readerDropped;

    public int Handle { get; } = handle;
    public StreamVTable VTable { get; private set; } = vTable;

    // TODO: Generate per type for this instrinsic.
    public Task Write()
    {
        // TODO: Generate for the interop name.
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        // TODO: for void streams, what should this do?
        var status = new WaitableStatus(VTable.Write(Handle, IntPtr.Zero, 0));
        if (status.IsBlocked)
        {
            //TODO: store somewhere so we can complete it later.
            var tcs = new TaskCompletionSource();
            return tcs.Task;
        }

        return Task.CompletedTask;
    }

    void IFutureStream.FreeBuffer()
    {
        bufferHandle.Free();
    }

    void IFutureStream.OtherSideDropped()
    {
        readerDropped = true;
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

    ~StreamWriter()
    {
        Dispose(false);
    }
}

public class StreamWriter<T>(int handle, StreamVTable vTable) : IFutureStream
{
    private GCHandle bufferHandle;
    private bool readerDropped;
    public int Handle { get; } = handle;
    public StreamVTable VTable { get; private set; } = vTable;

    // TODO: Generate per type for this instrinsic.
    public Task<int> Write(T[] payload)
    {
        // TODO: Generate for the interop name.
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        if (readerDropped)
        {
            throw new StreamDroppedException();    
        }

        if (VTable.Lower == null)
        {
            bufferHandle = GCHandle.Alloc(payload, GCHandleType.Pinned);
        }
        else
        {
            // Lower the payload
            throw new NotSupportedException("StreamWriter Write where the payload must be lowered.");
            // var loweredPayload = VTable.Lower(payload);
        }
        var status = new WaitableStatus(VTable.Write(Handle, bufferHandle.AddrOfPinnedObject(), (uint)payload.Length));
        if (status.IsBlocked)
        {
            var tcs = new TaskCompletionSource<int>();
            Console.WriteLine("blocked write");
            if(AsyncSupport.WaitableSet == null)
            {
                AsyncSupport.WaitableSet = AsyncSupport.WaitableSetNew();
            }
            Console.WriteLine("blocked write before join");
            AsyncSupport.Join(this, AsyncSupport.WaitableSet, new WaitableInfoState(tcs, this));
            Console.WriteLine("blocked write after join");
            return tcs.Task;
        }

        if (status.IsCompleted)
        {
            bufferHandle.Free();
            return Task.FromResult((int)status.Count);
        }

        throw new NotImplementedException($"Unsupported write status {status.State}");
    }

    void IFutureStream.FreeBuffer()
    {
        bufferHandle.Free();
    }

    void IFutureStream.OtherSideDropped()
    {
        readerDropped = true;
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

    ~StreamWriter()
    {
        Dispose(false);
    }
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
