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

    protected abstract int Write(int handle, IntPtr buffer);
}
