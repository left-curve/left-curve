// TODO: remove this linter exception once we add signed number types.
#[allow(dead_code)]
pub(crate) const fn grow_be_int<const INPUT_SIZE: usize, const OUTPUT_SIZE: usize>(
    input: [u8; INPUT_SIZE],
) -> [u8; OUTPUT_SIZE] {
    debug_assert!(INPUT_SIZE <= OUTPUT_SIZE);

    // check if sign bit is set
    let mut output = if input[0] & 0b10000000 != 0 {
        // negative number is filled up with 1s
        [0b11111111u8; OUTPUT_SIZE]
    } else {
        [0u8; OUTPUT_SIZE]
    };
    let mut i = 0;

    // copy input to the end of output
    // copy_from_slice is not const, so we have to do this manually
    while i < INPUT_SIZE {
        output[OUTPUT_SIZE - INPUT_SIZE + i] = input[i];
        i += 1;
    }

    output
}

// TODO: remove this linter exception once we add signed number types.
#[allow(dead_code)]
pub(crate) const fn grow_le_int<const INPUT_SIZE: usize, const OUTPUT_SIZE: usize>(
    input: [u8; INPUT_SIZE],
) -> [u8; OUTPUT_SIZE] {
    debug_assert!(INPUT_SIZE <= OUTPUT_SIZE);

    // check if sign bit is set
    let mut output = if input[INPUT_SIZE - 1] & 0b10000000 != 0 {
        // negative number is filled up with 1s
        [0b11111111u8; OUTPUT_SIZE]
    } else {
        [0u8; OUTPUT_SIZE]
    };
    let mut i = 0;

    // copy input to the beginning of output
    // copy_from_slice is not const, so we have to do this manually
    while i < INPUT_SIZE {
        output[i] = input[i];
        i += 1;
    }

    output
}

pub(crate) const fn grow_be_uint<const INPUT_SIZE: usize, const OUTPUT_SIZE: usize>(
    input: [u8; INPUT_SIZE],
) -> [u8; OUTPUT_SIZE] {
    debug_assert!(INPUT_SIZE <= OUTPUT_SIZE);

    let mut output = [0u8; OUTPUT_SIZE];
    let mut i = 0;

    // copy input to the end of output
    // copy_from_slice is not const, so we have to do this manually
    while i < INPUT_SIZE {
        output[OUTPUT_SIZE - INPUT_SIZE + i] = input[i];
        i += 1;
    }

    output
}

pub(crate) const fn grow_le_uint<const INPUT_SIZE: usize, const OUTPUT_SIZE: usize>(
    input: [u8; INPUT_SIZE],
) -> [u8; OUTPUT_SIZE] {
    debug_assert!(INPUT_SIZE <= OUTPUT_SIZE);

    let mut output = [0u8; OUTPUT_SIZE];
    let mut i = 0;

    // copy input to the beginning of output
    // copy_from_slice is not const, so we have to do this manually
    while i < INPUT_SIZE {
        output[i] = input[i];
        i += 1;
    }

    output
}

pub(crate) const fn bytes_to_digits<const S: usize, const OUT_SIZE: usize>(
    data: [u8; S],
) -> [u64; OUT_SIZE] {
    let mut result = [0u64; OUT_SIZE];
    let mut i = 0;
    while i < OUT_SIZE {
        result[i] = (data[i * 8] as u64)
            | ((data[i * 8 + 1] as u64) << 8)
            | ((data[i * 8 + 2] as u64) << 16)
            | ((data[i * 8 + 3] as u64) << 24)
            | ((data[i * 8 + 4] as u64) << 32)
            | ((data[i * 8 + 5] as u64) << 40)
            | ((data[i * 8 + 6] as u64) << 48)
            | ((data[i * 8 + 7] as u64) << 56);
        i += 1;
    }
    result
}
