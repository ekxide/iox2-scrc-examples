Iceoryx2 Rust SCRC Examples
===

A collection of examples demonstrating potential dark spots in the current Rust language specification.

We are not confident that these are all real issues, but they are subtle enough that it is not obvious from the current language spec whether we handle all of them correctly.

The first four examples link to accompanying code samples that can be run in isolation to observe the presented issues.

The remaining examples are more difficult to demonstrate from the user side, as they occur deep within the implementation of iceoryx2, so they only reference the respective sections in the iceoryx2 source code but do not come with specific example code.


Introduction to iceoryx2
---

[Iceoryx2](https://github.com/eclipse-iceoryx/iceoryx2/) is a zero-copy interprocess communication middleware for use in performance-sensitive, safety-critical systems. [Iceoryx utilizes shared memory for communication](https://ekxide.github.io/iceoryx2-book/main/fundamentals/shared-memory.html), meaning data exchanged between processes is only written to memory once, and that memory segment is then shared between processes.

For the examples below, we will use [the publish-subscribe messaging pattern](https://ekxide.github.io/iceoryx2-book/main/fundamentals/messaging-patterns/publish-subscribe.html) of iceoryx. This allows a single publisher to provide data for an arbitrary number of subscribers.

Before diving into the examples below, take a look at [the simplest publish/subscribe example in the iceoryx2 repo](https://github.com/eclipse-iceoryx/iceoryx2/tree/main/examples/rust/publish_subscribe) to get a feel for the API.


[Example #1 - Hidden Aliasing Through Shared Memory](ex1_subscriber_aliasing/src/main.rs)
---

This example demonstrates a fundamental issue with shared memory: Two subscribers subscribing to the same message will map the respective shared memory segment twice at different addresses, allowing the program to operate on aliased data without Rust being aware of the aliasing.

The program prints the data received through each of the two subscribers, along with the addresses where that data is stored. While it looks to the program (and analysis tools) as if there were two separate copies of the `PayloadData` in memory, these are in fact backed by the same physical memory mapped at two different addresses into the program's address space.

The whole point of iceoryx2 is to ensure that the access to this memory is safe. In particular, it is not possible for the program to violate Rust's single-mutable-reference rule.

However, the *implementation* of iceoryx is very much depending on the fact that it can hold multiple references to this memory at the same time. Ensuring that it can do so without leaving Rust in an invalid state when transitioning out of an unsafe block within the iceoryx2 implementation is essential.

### [Full Breakdown for Example #1](ex1_subscriber_aliasing/src/syscall.rs)

This is a full breakdown of the syscalls that iceoryx2 performs on a POSIX system for Example #1. On Windows, the underlying syscalls will be different, but the expectations of the surrounding iceoryx2 code will be largely unchanged.

* Each participant in the communication opens up a named shared memory segment using [`shm_open`](https://pubs.opengroup.org/onlinepubs/009695099/functions/shm_open.html). The call to `shm_open` returns a file descriptor for the shared memory segment. This call is probably not problematic.
* Each participant maps the shared memory region using [`mmap`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/mmap.html). The call to `mmap` returns a mutable pointer to the underlying memory region. This establishes the problematic aliasing that is the heart of the example.
* The publisher uses unsafe pointer arithmetic to obtain a pointer to the payload data in shared memory
* The publisher uses an unsafe pointer write call to write the payload
* Subscribers use unsafe pointer arithmetic to obtain a pointer to the payload in shared memory. The pointer values obtained by each participant will be different, even though they map to the same physical memory location.
* Subscribers will use an unsafe cast to obtain a reference to the `PayloadData` object. At this point, the object needs to be within its lifetime from the perspective of the subscriber.


[Example #2 - Spooky Action](ex2_subscriber_spooky_action/src/main.rs)
---

This example highlights a particularly problematic aspect of the aliasing from example #1.

The example contains a single publisher and a single subscriber. After exchanging a single message, the publisher will keep sending examples until it observes that the memory from the very first message is being reused for communication.

Consider the publisher's perspective on what happens here: The publisher reads data from a memory address. Then it "looks away" for a while, and when it receives the next message at that address, the contents of the memory have changed, even though there were *no writes* to that memory location from within the process.

This is obviously true if the publisher resides in a different process, but is also the case for this single process example, because the publisher mapped the memory segment at a different memory address than the subscriber, again creating a hidden aliasing similar to the situation in example #1.

While this example may look somewhat harmless from the user code, as the user needs to drop the received sample for it to become eligible for reuse, keep in mind that the *implementation* again keeps hold of the underlying memory throughout.

This situation is similar to that of memory-mapped I/O, where contents of memory can just change without any action from the program itself. But it is particularly troublesome here, since a write at one address initiated by a program will directly impact memory at multiple other mapped addresses, violating the usual assumptions about aliasing.

Additionally, it must be noted that for this interaction to play out correctly, the publisher and subscriber [need to *synchronize over atomics* placed in the shared memory segment](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-cal/src/zero_copy_connection/common.rs#L166). So it is *not* sufficient to use volatile access for performing the reads, as required for memory-mapped I/O, it must be possible to have full support for atomic memory operations. Fully volatile writes would also be a performance pessimization for this use case, as they prevent optimizing redundant writes to the same memory location.

### [Full Breakdown for Example #2](ex2_subscriber_spooky_action/src/syscall.rs)

This is a full breakdown of the syscalls that iceoryx2 performs on a POSIX system for Example #2. On Windows, the underlying syscalls will be different, but the expectations of the surrounding iceoryx2 code will be largely unchanged.

* As in example #1, each participant uses calls to `shm_open` and `mmap` to map the shared memory segment.
* As in example #1, subscribers and publishers use unsafe pointer arithmetic, unsafe pointer writes, and unsafe pointer dereferencing.
* In addition to the code from example #1, this example includes the bookeeping data structure `ShmMgmt`, which is used for coordinating between publishers and subscribers. This is a highly simplified version of the management data structure that is used by iceoryx2.
* In the example, the publisher writes to an atomic counter that indicates the currently active payload. Writes by the publisher synchronize with concurrent reads by the subscribers through this counter. All synchronization guarantees given by the memory model for shared atomics are expected to work in this scenario, even though the shared atomic resides at different addresses in memory.
* Payload data is wrapped in a `MaybeUninit`. Subscribers use an unsafe `assume_init_ref` call to obtain references to the Payload data. The publisher guarantees that the object was initialized properly and synchronizes with the subscriber via the atomic counter. However, the initialized object was constructed by the publisher at a different address than the subscriber uses for reading (or, in the case of separate processes, was not created within the publisher program at all).
* In the example, only a single writer ever modifies the memory. While this is also true for a simple publisher/subscriber scenario with iceoryx2, other messaging patterns may require multiple participants to write to the same memory location as well.

[Example #3 Provenance of objects in shared memory](ex3_provenance/src/main.rs)
---

The [`ZeroCopySend` trait](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-bb/elementary-traits/src/zero_copy_send.rs#L32) is used as an opt-in to mark types as eligible to be send over iceoryx. The primary restriction imposed on payloads is that their memory layout must be suitably predictable, but otherwise Payloads can be arbitrary powerful iceoryx2 objects.

In particular, we need to be able to support use cases as the one demonstrated in this example, where a publisher receives a payload and then directly calls a function on a payload type in shared memory. It is not acceptable for our clients to enforce an additional deserialization step, objects must be directly accessible from shared memory.

This creates potential problems for provenance. The implementation simply maps a segment of memory into the process' address space and then [starts casting pointers to object pointers to start the lifetime of those objects](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-cal/src/dynamic_storage/posix_shared_memory.rs#L562).

It must be ensured that:

* Pointers derived from the base pointer of a shared memory segment provide the correct provenance so that pointers to payload objects can be obtained from it.
* Objects pulled in via a shared memory mapping have the correct lifetime so that it is safe for user code to access them as if they had been created within the program.


[Example #4 - Referencing object addresses in shared memory](ex4_relocatable_pointer/src/main.rs)
---
Because the base address of the shared memory segment is different for every client, an object in shared memory cannot refer to another object in shared memory by pointer. Node-based data structures like trees or linked lists that get placed in shared memory cannot use pointers, because the pointer values would no longer be valid when evaluated on the receiving side.

Instead of using absolute addresses to identify object locations, relative offsets are used. These can be implemented either as

* Offset from the base of the shared memory segment; or
* Offset from the address of the pointer itself.

Iceoryx implements the second option in its [`RelocatablePointer` type](https://github.com/eclipse-iceoryx/iceoryx2/blob/main/iceoryx2-bb/elementary/src/relocatable_ptr.rs). The advantage over the first option is that this pointer can be created without knowing the base address of the memory segment, which is only obtainable through the iceoryx2 backend, while a `RelocatablePointer` can be created by the user from just the addresses of the pointer object and its pointee.

The `RelocatablePointer` is used throughout the implementation of iceoryx2, but this example shows a use of it that is the closest to the user-facing API surface: The `iceoryx2_bb_container` support library provides the [`FixedSizeQueue` type](https://github.com/eclipse-iceoryx/iceoryx2/blob/main/iceoryx2-bb/container/src/queue.rs) for use with iceoryx2 payloads. This type [uses a `RelocatablePointer` in its implementation](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-bb/container/src/queue.rs#L234), as can be easily observed by inspecting the queue object `q` in a debugger.

The implementation of `RelocatablePointer` now again triggers an issue of pointer provenance. The order of operations for a receiving client are as follows:

* A client maps a memory segment.
* It obtains a reference to a `RelocatablePointer` within that segment.
* From the `RelocatablePointer` it obtains a reference to the pointee object by adding the address of the `RelocatablePointer` itself with its stored offset and casting the resulting pointer value to the pointee type.

We believe this to be a different, more difficult issue than the provenance concerns from example #3, as it involves obtaining a reference for the pointee type from a reference to a `RelocatablePointer` *without* again going through the original base pointer of the address. So the `self` pointer of the `RelocatablePointer` object needs to preserve the necessary provenance so that it can be used to obtain the final pointer to the pointee.

[The relevant operation in the implementation of `RelocatablePointer`](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-bb/elementary/src/relocatable_ptr.rs#L147).


[Example #5 - Benign use of uninitialized memory](ex5_uninitialized_memory/src/main.rs)
---

Iceoryx2 does not require users to manually synchronize startup of different clients. This requires the implementation to correctly handle situations where multiple publishers or subscribers attempt to create a communication channel concurrently at startup.

To correctly detect race conditions in such scenarios, processes observe the contents of shared memory to watch for activity by other processes. The first process to create a shared memory segment is responsible for correctly initializing the necessary data structures. Once it is finished doing so, it will write a specific bit pattern at a fixed offset in the data segment.

When a process opens an existing shared memory segment, it will check for that bit pattern. If the pattern is not there, it means that the creating process is still in the process of setting up the memory, so the opening process will wait for the bit pattern to appear before continuing.

The problem with this approach is that [the first read to check for that pattern may read uninitialized memory](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-cal/src/dynamic_storage/posix_shared_memory.rs#L254). We consider this a benign case of uninitialized read, as the reading process will only use it for the purpose of detecting this case. On platforms where mapped memory is zero initialized by the operating system it is also very much clear what the result of such an initialized read is going to be.

Still, technically, the process is reading uninitialized memory. This is also different from the case in example #3, as it not only reads memory that has not been initialized by the process that performs the read, but it has not been initialized by *anyone* except maybe the operating system.

We are not aware of mechanisms in the Rust language that allow blessing of such uninitialized reads.


Example #6 - Sequence Locks
---

This is a somewhat famous gap in the C++ memory model, with plenty of prior work. See for example:

- WG21 [P1478 - Byte-wise atomic memcpy](https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2022/p1478r7.html)
- Rust [RFC AtomicPerByte](https://github.com/rust-lang/rfcs/pull/3301)

Implementing [sequence locks](https://en.wikipedia.org/wiki/Seqlock), a prominent implementation technique for reader-writer-locks, requires relying on a form of benign data race. A reader may optimistically read data from memory without synchronization. After the read it will perform a synchronized access to ensure that no data race occurred during the unsynchronized read. If so, the data can be used safely, otherwise the data is discarded and the reader tries again.

The problem is that the initial unsynchronized read may trigger a data race. The fact that the data will be discarded afterwards does not mitigate this.

Iceoryx2 implements this pattern [in its `UnrestrictedAtomic` type](https://github.com/eclipse-iceoryx/iceoryx2/blob/ff95bcdb9697b1eb9117394171b71ce952575181/iceoryx2-bb/lock-free/src/spmc/unrestricted_atomic.rs#L282) which is used extensively for internal synchronization.


Example #7 - Benign reads of padding bytes
---

While iceoryx in general aims to never copy its data, we also deal with scenarios where a client application needs to send data over a gateway. In general, such a case may require full serialization, but it is often sufficient to simply dump the raw bits of an object into a network socket and reconstruct the object on the other side from that.

In the simplest case, one would just `copy_nonoverlapping` the entire size of the data payload into a buffer for sending. The problem is that this data payload may contain padding bytes, which are fine to memcopy, but any subsequent attempt to interpret the byte values of the target buffer corresponding to the padding bytes will be considered access of an unitialized value.

This is particularly problematic when using such data in conjunction with facilities that require them to be valid `u8` data, such as external communication APIs or (worse) atomics.

We fear that the current semantics in that regard may be too strict and might needlessly pessimize useful algorithms.

