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
    public int Handle => (int)(status >> 4);
    public bool IsStarting => State == 0;
    public bool IsStarted => State == 1;
    public bool IsReturned => State == 2;
    public bool IsStartedCancelled => State == 3;
    public bool IsReturnedCancelled => State == 4;
}

public readonly struct EventWaitable
{
    public EventWaitable(EventCode eventCode, uint waitable, uint code)
    {
        Console.WriteLine($"EventWaitable with code {code}");
        EventCode = eventCode;
        Waitable = (int)waitable;
        Code = code;
        
        if(eventCode == EventCode.Subtask)
        {
            IsSubtask = true;
            SubtaskStatus = new SubtaskStatus(code);
        }
        else
        {
            WaitableStatus = new WaitableStatus(code);
        }
    }

    public readonly EventCode EventCode;
    public readonly int Waitable;
    public readonly uint Code;

    public bool IsSubtask { get; }
    public readonly WaitableStatus WaitableStatus;
    public readonly SubtaskStatus SubtaskStatus;
    public readonly int WaitableCount => (int)Code >> 4;
    public bool IsDropped => !IsSubtask && WaitableStatus.IsDropped;
    public bool IsCompleted => IsSubtask && SubtaskStatus.IsReturned || !IsSubtask && WaitableStatus.IsCompleted;
}

