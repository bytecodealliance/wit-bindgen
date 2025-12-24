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
 * Helpers for future support.
 */
public delegate ulong New();
public delegate int StartRead(int handle, IntPtr buffer);
public delegate void DropReader(int handle);
public delegate void DropWriter(int handle);
public delegate int Write(int handle, IntPtr buffer);

public struct FutureVTable
{
    public New New;
    public StartRead StartRead;
    public Write Write;
    public DropReader DropReader;
    public DropWriter DropWriter;
}

public static class FutureHelpers
{
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
}

public class FutureReader(int handle, FutureVTable vTable) : IDisposable // : TODO Waitable
{
    public int Handle { get; private set; } = handle;
    public FutureVTable VTable { get; private set; } = vTable;

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

        var status = new WaitableStatus(vTable.StartRead(Handle, IntPtr.Zero));
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
        vTable.DropReader(Handle);
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

        var status = new WaitableStatus(vTable.StartRead(Handle, IntPtr.Zero));
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
        vTable.DropReader(Handle);
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
        VTable.DropWriter(Handle);
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
        VTable.DropWriter(Handle);
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
