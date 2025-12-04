/**
 * Helpers for future reader support.
 */

public abstract class FutureReader(int handle) : IDisposable // : TODO Waitable
{
    public int Handle { get; } = handle;

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