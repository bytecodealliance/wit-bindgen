use std::alloc::Layout;

// dummy to just make the generated code compile, for now
pub unsafe trait Subtask {
    type ABI_LAYOUT: Layout;
}
