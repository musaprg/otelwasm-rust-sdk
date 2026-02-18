use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::status::Status;

static PINNED_ALLOCATIONS: Lazy<Mutex<HashMap<i32, Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn allocate(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    let mut buf = vec![0_u8; size as usize];
    let ptr = buf.as_mut_ptr() as i32;
    let mut allocations = PINNED_ALLOCATIONS
        .lock()
        .expect("allocation mutex poisoned");
    allocations.insert(ptr, buf);
    ptr
}

pub fn take_ownership(ptr: i32, size: i32) -> Result<Vec<u8>, Status> {
    if ptr == 0 && size == 0 {
        return Ok(Vec::new());
    }
    if ptr == 0 || size < 0 {
        return Err(Status::error("invalid pointer/size"));
    }

    let mut allocations = PINNED_ALLOCATIONS
        .lock()
        .expect("allocation mutex poisoned");
    let mut buf = allocations
        .remove(&ptr)
        .ok_or_else(|| Status::error(format!("unknown allocation pointer: {ptr}")))?;
    if size as usize > buf.len() {
        return Err(Status::error(format!(
            "payload size {size} exceeds allocation size {}",
            buf.len()
        )));
    }
    buf.truncate(size as usize);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_ownership_accepts_high_bit_pointer_pattern() {
        let ptr = -1_i32;
        let mut allocations = PINNED_ALLOCATIONS
            .lock()
            .expect("allocation mutex poisoned");
        allocations.insert(ptr, vec![1, 2, 3]);
        drop(allocations);

        let got = take_ownership(ptr, 3).expect("take ownership should succeed");
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn take_ownership_rejects_size_larger_than_alloc() {
        let ptr = 7_i32;
        let mut allocations = PINNED_ALLOCATIONS
            .lock()
            .expect("allocation mutex poisoned");
        allocations.insert(ptr, vec![1, 2, 3]);
        drop(allocations);

        let err = take_ownership(ptr, 4).expect_err("size overflow must error");
        assert_eq!(err.code, crate::StatusCode::Error);
    }
}
