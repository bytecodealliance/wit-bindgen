/**
 * Helpers for future support.
 */

public class WaitableStatus (int status)
{
    public int State => status & 0xf;
    public int Count => (int)(status >> 4);
    public bool IsBlocked => status == -1;
    public bool IsCompleted => State == 0;
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

[System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
public ref struct EventWaitable
{
    public EventCode EventCode;
    int Waitable;
    int Code;
}

public class WaitableSet(int handle)
{
    public int Handle { get; } = handle;
}