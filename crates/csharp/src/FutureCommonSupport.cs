/**
 * Helpers for future support.
 */

public class WaitableStatus (int status)
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

public struct EventWaitable
{
    public EventWaitable(EventCode eventCode, int code)
    {
        Event = eventCode;
        Status = new WaitableStatus(code);
    }
    public EventCode Event;
    public int Waitable;
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