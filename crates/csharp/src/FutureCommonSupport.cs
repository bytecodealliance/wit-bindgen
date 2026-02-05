/**
 * Helpers for future support.
 */

public readonly struct WaitableStatus (uint status)
{
    public uint State => status & 0xf;
    public uint Count => status >> 4;
    public bool IsBlocked => status == 0xffffffff;
    public bool IsCompleted => State == 0;
    public bool IsDropped => State == 1;
    public bool IsCancelled => State == 2;
}

public readonly struct SubtaskStatus (uint status)
{
    public uint State => status & 0xf;
    public uint Handle => status >> 4;
    public bool IsSubtaskStarting => State == 0;
    public bool IsSubtaskStarted => State == 1;
    public bool IsSubtaskReturned => State == 2;
    public bool IsSubtaskStartedCancelled => State == 3;
    public bool IsSubtaskReturnedCancelled => State == 4;
}

public readonly struct EventWaitable
{
    public EventWaitable(EventCode eventCode, uint waitable, uint code)
    {
        Event = eventCode;
        Waitable = waitable;
        // TODO: create distinguished waitables depending on the code?
        if(eventCode == EventCode.Subtask)
        {
            SubTaskStatus = new SubtaskStatus(code);
        }
        else
        {
            Status = new WaitableStatus(code);
        }
    }

    public readonly EventCode Event;
    public readonly uint Waitable;
    public readonly uint Code;

    public readonly WaitableStatus Status;
    public readonly SubtaskStatus SubTaskStatus;
}

