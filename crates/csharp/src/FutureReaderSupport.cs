/**
 * Helpers for future reader support.
 */

public abstract class FutureReader(int handle) // : TODO Waitable
{
    public int Handle { get; } = handle;

    // TODO: Generate per type for this instrinsic.
    public Task Read()
    {
        // TODO: Generate for the interop name and the namespace.

        var status = new WaitableStatus(Read(Handle));
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

    protected abstract int Read(int handle);
}