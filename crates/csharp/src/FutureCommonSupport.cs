/**
 * Helpers for future support.
 */

public readonly struct WaitableStatus (int status)
{
    public int State => status & 0xf;
    public int Count => (int)(status >> 4);
    public bool IsBlocked => status == -1;
    public bool IsCompleted => State == 0;
    public bool IsDropped => State == 1;
}

public enum EventCode
{
    None,
    Subtask,
    StreamRead,
    StreamWrite,
    FutureRead,
    FutureWrite,
    Cancel,
}

public readonly struct EventWaitable
{
    public EventWaitable(EventCode eventCode, int waitable, int code)
    {
        Event = eventCode;
        Waitable = waitable;
        Status = new WaitableStatus(code);
    }
    public readonly EventCode Event;
    public readonly int Waitable;
    public readonly int Code;

    public readonly WaitableStatus Status;
}

public partial class WaitableSet(int handle) : IDisposable
{
    public int Handle { get; } = handle;

    void Dispose(bool _disposing)
    {
        {{interop_name}}.WaitableSetDrop(Handle);
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