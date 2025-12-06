/**
 * Helpers for the async support.
 */

public enum CallbackCode
{
    Exit = 0,
    Yield = 1,
}

public partial class WaitableSet(int handle) : IDisposable
{
    public int Handle { get; } = handle;

    void Dispose(bool _disposing)
    {
        AsyncSupport.WaitableSetDrop(this);
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
        [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-set-new]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static extern int WaitableSetNew();

        [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-join]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static extern void WaitableJoin(int waitable, int set);

        [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-set-wait]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern int WaitableSetWait(int waitable, int* waitableHandlePtr);

        [global::System.Runtime.InteropServices.DllImportAttribute("$root", EntryPoint = "[waitable-set-drop]"), global::System.Runtime.InteropServices.WasmImportLinkageAttribute]
        internal static unsafe extern void WaitableSetDrop(int waitable);
    }

    public static WaitableSet WaitableSetNew() 
    {{
        var waitable = Interop.WaitableSetNew();
        return new WaitableSet(waitable);
    }}

    public static void Join(FutureWriter writer, WaitableSet set) 
    {{
        Interop.WaitableJoin(writer.Handle, set.Handle);
    }}

    public unsafe static EventWaitable WaitableSetWait(WaitableSet set) 
    {{
        int* buffer = stackalloc int[2];
        var eventCode = (EventCode)Interop.WaitableSetWait(set.Handle, buffer);
        return new EventWaitable(eventCode, buffer[0], buffer[1]);
    }}

    public static void WaitableSetDrop(WaitableSet set) 
    {{
        Interop.WaitableSetDrop(set.Handle);
    }}
}

/**
 * Helpers for future reader support.
 */
public abstract class FutureReader(int handle) : IDisposable // : TODO Waitable
{
    public int Handle { get; private set; } = handle;

    public int TakeHandle()
    {
        if(Handle == 0)
        {
            throw new InvalidOperationException("Handle already taken");
        }
        var handle = Handle;
        Handle = 0;
        return handle;
    }

    // TODO: Generate per type for this instrinsic.
    public Task Read()
    {
        // TODO: Generate for the interop name and the namespace.

        var status = new WaitableStatus(ReadInternal());
        if (status.IsBlocked)
        {
            //TODO: store somewhere so we can complete it later.
            var tcs = new TaskCompletionSource();

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
        Drop();
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

    protected abstract int ReadInternal();
    protected abstract void Drop();
}

/**
 * Helpers for future writer support.
 */
public abstract class FutureWriter(int handle) // : TODO Waitable
{
    public int Handle { get; } = handle;

    // TODO: Generate per type for this instrinsic.
    public Task Write()
    {
        // TODO: Generate for the interop name.
        var status = new WaitableStatus(Write(Handle, IntPtr.Zero));
        if (status.IsBlocked)
        {
            //TODO: store somewhere so we can complete it later.
            var tcs = new TaskCompletionSource();
            return tcs.Task;
        }

        throw new NotImplementedException();
    }

    protected abstract void Drop();

    void Dispose(bool _disposing)
    {
        // Free unmanaged resources if any.
        Drop();
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
    
    protected abstract int Write(int handle, IntPtr buffer);
}
