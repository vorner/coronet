# Coronet

Assorted coroutine utilities, abusing Rust async/await under the hood.

Rust has generators as an unstable feature. They would allow writing iterators,
parsers, and many other such things conveniently, but hey, they are unstable and
likely to change.

But they are also used internally by the compiler to create async functions.
Could the async functions be abused to give us the generators superpowers?

For now, this crate is somewhat *experimental*. Expect rough edges, weird error
messages, things changing between releases and such (but I intend to minimize
the use of `unsafe` and stick to things that I believe to work correctly once
they compile).
