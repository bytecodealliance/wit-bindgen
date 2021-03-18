# wasmtime

* Functions need to be able to at least optionally return a trap, e.g.
  `proc_raise` or they were passed an invalid buffer.

* buffer-in-buffer doesn't work. Doesn't work because we can't get a re-access
  of the transaction to add more buffers into it after-the-fact.
