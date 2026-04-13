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
    public static unsafe uint Callback(EventWaitable e, ContextTask* contextPtr)
    {
        ContextTask* contextTaskPtr = ContextGet();

        var waitables = pendingTasks[contextTaskPtr->WaitableSetHandle];
        var waitableInfoState = waitables[e.Waitable];

        if (e.IsDropped)
        {
            waitableInfoState.FutureStream.OtherSideDropped();
        }

        if (e.IsCompleted || e.IsDropped)
        {
            // The operation is complete so we can free the buffer and remove the waitable from our dicitonary
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
                ContextSet(null);
                Marshal.FreeHGlobal((IntPtr)contextTaskPtr);
                return (uint)CallbackCode.Exit;
            }

            return (uint)CallbackCode.Wait | (uint)(contextTaskPtr->WaitableSetHandle << 4);
        }

        throw new NotImplementedException($"WaitableStatus not implemented {e.WaitableStatus.State} in set {contextTaskPtr->WaitableSetHandle}");
    }

    // This method is unsafe because we are using unmanaged memory to store the context.
    internal static unsafe Task TaskFromStatus(uint status)
    {
        var subtaskStatus = new SubtaskStatus(status);
        status = status & 0xF;

        var tcs = new TaskCompletionSource<int>();
        if (subtaskStatus.IsSubtaskStarting || subtaskStatus.IsSubtaskStarted)
        {
            ContextTask* contextTaskPtr = ContextGet();
            if (contextTaskPtr == null)
            {
                contextTaskPtr = AllocateAndSetNewContext();
            }

            Join(subtaskStatus.Handle, contextTaskPtr->WaitableSetHandle, new WaitableInfoState(tcs));

            return tcs.Task;
        }
        else if (subtaskStatus.IsSubtaskReturned)
        {
            tcs.SetResult(0);
            return Task.CompletedTask;
        }
        else
        {
            throw new Exception($"unexpected subtask status: {status}");
        }
    }

    // unsafe because we are using pointers.
    public static unsafe Task<T> TaskFromStatus<T>(uint status, Func<T> liftFunc)
    {
        var subtaskStatus = new SubtaskStatus(status);

        if (subtaskStatus.IsSubtaskStarting || subtaskStatus.IsSubtaskStarted)
        {
            ContextTask* contextTaskPtr = ContextGet();
            if (contextTaskPtr == null) {
                contextTaskPtr = AllocateAndSetNewContext();
            }

            var intTaskCompletionSource = new TaskCompletionSource<int>();
            var tcs = new LiftingTaskCompletionSource<T>(intTaskCompletionSource, liftFunc);
            Join(subtaskStatus.Handle, contextTaskPtr->WaitableSetHandle, new WaitableInfoState(intTaskCompletionSource));

            return tcs.Task;
        }
        else if (subtaskStatus.IsSubtaskReturned)
        {
            var tcs = new TaskCompletionSource<T>();
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
                throw new NotImplementedException("lifting results from async functions not implemented yet");
            });
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
public delegate void Lower(object payload, uint size);
public delegate uint CancelRead(int handle);
public delegate uint CancelWrite(int handle);

public interface ICancelableRead
{
    uint CancelRead(int handle);
}

public interface ICancelableWrite
{
    uint CancelWrite(int handle);
}

public interface ICancelable
{
    uint Cancel();
}

public class CancelableRead(ICancelableRead cancelableVTable, int handle) : ICancelable
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

public struct FutureVTable : ICancelableRead, ICancelableWrite
{
    internal New New;
    internal FutureRead Read;
    internal FutureWrite Write;
    internal DropReader DropReader;
    internal DropWriter DropWriter;
    internal Lower? Lower;
    internal CancelWrite CancelWriteDelegate;
    internal CancelRead CancelReadDelegate;

    public uint CancelRead(int handle)
    {
        return CancelReadDelegate(handle);
    }

    public uint CancelWrite(int handle)
    {
        return CancelWriteDelegate(handle);
    }
}

public struct StreamVTable : ICancelableRead, ICancelableWrite
{
    internal New New;
    internal StreamRead Read;
    internal StreamWrite Write;
    internal DropReader DropReader;
    internal DropWriter DropWriter;
    internal Lower? Lower;
    internal CancelWrite CancelWriteDelegate;
    internal CancelRead CancelReadDelegate;

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

    protected GCHandle LiftBuffer<T>(T[] buffer)
    {
        // For primitive, blittable types
        if (typeof(T).IsPrimitive || typeof(T).IsValueType)
        {
            return GCHandle.Alloc(buffer, GCHandleType.Pinned);
        }
        else
        {
            // TODO: create buffers for lowered stream types and then lift
            throw new NotImplementedException("reading from futures types that require lifting");
        }
    }

    internal abstract uint VTableRead(IntPtr bufferPtr, int length);

    // unsafe as we are working with pointers.
    internal unsafe ComponentTask<int> ReadInternal(Func<GCHandle?> liftBuffer, int length, ICancelableRead cancelableRead)
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
            var task = new ComponentTask<int>(new CancelableRead(cancelableRead, Handle));
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
            return ComponentTask<int>.FromResult((int)status.Count);
        }

        throw new NotImplementedException(status.State.ToString());
    }

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

    public ComponentTask Read()
    {
        return ReadInternal(() => null, 0, VTable);
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

public class FutureReader<T>(int handle, FutureVTable vTable) : ReaderBase(handle)
{
    public FutureVTable VTable { get; private set; } = vTable;

    public ComponentTask<T> Read()
    {
        T[] buf = new T[1];
        ComponentTask<int> internalTask = ReadInternal(() => LiftBuffer(buf), 1, VTable);

        // Wrap the task so we can return a T and not the number of Ts read
        ComponentTask<T> readTask = new(new DelegatingCancelable(internalTask));

        internalTask.ContinueWith(it =>
        {
            if (it.IsCompletedSuccessfully)
            {
                readTask.SetResult(buf[0]);
            }
            else if (!it.IsCanceled)
            {
                //TODO
                throw new NotImplementedException("faulted future read not implemented");
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
        return ReadInternal(() => null, length, VTable);
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

    public ComponentTask<int> Read(T[] buffer)
    {
        return ReadInternal(() => LiftBuffer(buffer), buffer.Length, VTable);
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
    private GCHandle? bufferHandle;
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
    internal unsafe ComponentTask<int> WriteInternal(Func<GCHandle?> lowerPayload, int length, ICancelableWrite cancelable)
    {
        if (Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }

        if (readerDropped)
        {
            throw new StreamDroppedException();    
        }
        bufferHandle = lowerPayload();

        var status = new WaitableStatus(VTableWrite(bufferHandle == null ? IntPtr.Zero : bufferHandle.Value.AddrOfPinnedObject(), length));
        canDrop = true;  // We can only call drop once something has been written.
        if (status.IsBlocked)
        {
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
            bufferHandle?.Free();
            return ComponentTask<int>.FromResult((int)status.Count);
        }

        throw new NotImplementedException($"Unsupported write status {status.State}");
    }

    void IFutureStream.FreeBuffer()
    {
        bufferHandle?.Free();
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
        return WriteInternal(() => null, 0, VTable);
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

    private GCHandle LowerPayload(T[] payload)
    {
        if (VTable.Lower == null)
        {
            return GCHandle.Alloc(payload, GCHandleType.Pinned);
        }
        else
        {
            // Lower the payload
            throw new NotSupportedException("StreamWriter Write where the payload must be lowered.");
            // var loweredPayload = VTable.Lower(payload);
        }
    }

    public ComponentTask<int> Write(T payload)
    {
        return WriteInternal(() => LowerPayload([payload]), 1, VTable);
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
        return WriteInternal(() => null, 0, VTable);
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
    private GCHandle bufferHandle;
    public StreamVTable VTable { get; private set; } = vTable;

    private GCHandle LowerPayload(T[] payload)
    {
        if (VTable.Lower == null)
        {
            return GCHandle.Alloc(payload, GCHandleType.Pinned);
        }
        else
        {
            // Lower the payload
            throw new NotSupportedException("StreamWriter Write where the payload must be lowered.");
            // var loweredPayload = VTable.Lower(payload);
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
    }

    internal void SetResult(int count)
    {
        if (taskCompletionSource != null)
        {
            taskCompletionSource.SetResult(count);
        }
        else if (componentTask != null)
        {
            componentTask.SetResult();
        }
        else if (componentTaskInt != null)
        {
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

    public static ComponentTask<T> FromResult<T>(T result)
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