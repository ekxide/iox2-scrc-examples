Stories for Unsafe Code Blocks
===

This is an attempt to use a [storytelling approach](https://www.ralfj.de/blog/2026/03/13/inline-asm.html) to argue why the unsafe blocks used in the code sample should be valid Rust code.

It should be noted that this argument is only sufficient to argue, why the code should be allowed to work, but still does not give a formal guarantee that a given implementation will behave as expected. In particular, we still require:

* A formal argument why a concrete implementation does not violate Rust's safety model
* An assurance that a concrete implementation will not be broken by existing or future compiler optimizations

Mapping memory
---

This is covered [as an example in the blog post](https://www.ralfj.de/blog/2026/03/13/inline-asm.html#page-table-manipulation).

Memory mapping is likened to a memory allocation. For the following sequence of commands:

```rust
let shm_fd_publisher =
    unsafe { libc::shm_open(/* ... */) };
unsafe { libc::ftruncate(shm_fd_publisher, shm_size as _) };
let shm_base_publisher: *mut ShmMgmt = unsafe {
    libc::mmap(/* ... */).cast()
};
unsafe {
    shm_base_publisher.write(ShmMgmt {
        counter: AtomicUsize::new(0),
        allocator_cells: [MaybeUninit::uninit(); 1024],
    })
};
```

The story is that of allocating a single `ShmMgmt` object:

```rust
let mut b = Box::new(ShmMgmt {
    counter: AtomicUsize::new(0),
    allocator_cells: [MaybeUninit::uninit(); 1024]
});
let shm_base_publisher: *mut ShmMgmt = &mut *b; 
```

Aliasing
---

Aliasing is also mentioned [as an example in the blog post](https://www.ralfj.de/blog/2026/03/13/inline-asm.html#page-table-manipulation-ii-duplicating-pages).

We adopt the example's approach of having the story code spawn multiple threads:

* The story for the allocation code is expanded to spawn a new thread that may concurrently write to the allocated memory segment
* Any write to a shared memory segment triggers an asynchronous write by the respective subscriber story thread

The story code is now full of potential data races. However, iceoryx2 requires synchronization for those anyway, as in an IPC context, these accesses really do happen on different physical threads in different physical processes. The additional story threads introduced now follow the same synchronization protocol as the physical threads would in an IPC scenario.

Note that under this model, the example 1 (subscriber aliasing) as written would have undefined behavior due to a data race between the main thread and the story threads. This is not a problem in case of iceoryx2, but may be problematic for other use cases.

Unmappping Memory
---

The story code for unmapping memory is deallocation of the memory allocated by the allocation code.

The following command:

```rust
unsafe { libc::shm_unlink(/* ... */) };
```

The story code is dropping the respective box object from the allocation story code:

```rust
drop(b);
```

Note that this only works for deallocating the entire memory. There [seems to be an open problem](https://github.com/llvm/llvm-project/pull/141338) how to deal with "shrinking memory" scenarios, where only parts of the mapped memory are unmapped. It is unclear how critical that issue is for our purposes, but we believe at least iceoryx2 not to be impacted by this limitation.

Open Questions
---

What are the boundaries for stories? Can a story span multiple unsafe blocks, or does each unsafe block need to be its own self-contained story?

Is there an isomorphism for mapping an n-block story to a single-block story? Something like, Block 1 spawns a virtual machine implementing the entirety of the story for a large single block in a separate thread and blocks 2..n exchange messages with that virtual machine in a thread safe way, exposing only the aspects of the virtual machine influenced by their block.

Safety
---

No formal safety requirements for libc. Expectations is that they behave "as specified by POSIX", but it is unclear what that means in the context of language semantics.

It is unclear how to formalize this properly and to what extent this falls into the domain of language semantics.
