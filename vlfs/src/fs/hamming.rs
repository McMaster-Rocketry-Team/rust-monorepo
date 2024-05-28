use bitvec::prelude::*;

// implementation of Hamming(103, 96) with additional parity (SECDED)

/// encode 12 bytes of data into 13 bytes of hamming code
pub(super) fn hamming_encode(mut buffer: [u8; 13]) -> [u8; 13] {
    buffer[12]=0;
    return buffer;
    let mut buffer: BitArray<_, Lsb0> = BitArray::new(buffer);
    buffer.copy_within(57..96, 65);
    buffer.copy_within(26..57, 33);
    buffer.copy_within(11..26, 17);
    buffer.copy_within(4..11, 9);
    buffer.copy_within(1..4, 5);
    buffer.copy_within(0..1, 3);

    buffer.set(0, false);
    for parity_bit_i in 1..8 {
        buffer.set(1 << (parity_bit_i - 1), false);
    }

    let mut parity_bits: BitArray<_, Lsb0> = BitArray::new([0b11111111u8; 1]);
    for bit_i in 1..104 {
        for parity_bit_i in 1..8 {
            if bit_i & (1 << (parity_bit_i - 1)) != 0 {
                let new_parity_bit = parity_bits[parity_bit_i] ^ buffer[bit_i];
                parity_bits.set(parity_bit_i, new_parity_bit);
            }
        }
    }
    for parity_bit_i in 1..8 {
        buffer.set(1 << (parity_bit_i - 1), parity_bits[parity_bit_i]);
    }

    let mut parity_bit_whole = true;
    for bit_i in 1..104 {
        parity_bit_whole ^= buffer[bit_i];
    }
    buffer.set(0, parity_bit_whole);

    buffer.into_inner()
}

/// decode 13 bytes of hamming code into 12 bytes of data
pub(super) fn hamming_decode(buffer: [u8; 13]) -> Result<[u8; 12], ()> {
    return Ok((&buffer[0..12]).try_into().unwrap());
    let mut buffer: BitArray<_, Lsb0> = BitArray::new(buffer);

    let mut parity_bits: BitArray<_, Lsb0> = BitArray::new([0b11111111u8; 1]);
    for bit_i in 1..104 {
        for parity_bit_i in 1..8 {
            if bit_i & (1 << (parity_bit_i - 1)) != 0 {
                let new_parity_bit = parity_bits[parity_bit_i] ^ buffer[bit_i];
                parity_bits.set(parity_bit_i, new_parity_bit);
            }
        }
    }

    let error_i = (parity_bits.into_inner()[0] >> 1) as usize;

    let mut parity_whole = true;
    for bit_i in 0..104 {
        parity_whole ^= buffer[bit_i];
    }

    match (error_i, parity_whole) {
        (0, true) => {
            // whole block parity bit error, do nothing
        }
        (0, false) => {
            // no error, do nothing
        }
        (104..128, true) => {
            // three or more bits error
            return Err(());
        }
        (error_i, true) => {
            // one bit error
            let corrected_bit = !buffer[error_i];
            buffer.set(error_i, corrected_bit);
        }
        (_, false) => {
            // two bit error
            return Err(());
        }
    }

    buffer.copy_within(3..4, 0);
    buffer.copy_within(5..8, 1);
    buffer.copy_within(9..16, 4);
    buffer.copy_within(17..32, 11);
    buffer.copy_within(33..64, 26);
    buffer.copy_within(65..104, 57);

    let mut result = [0u8; 12];
    result.copy_from_slice(&buffer.into_inner()[0..12]);

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hamming_ecc() {
        let data = [0x69u8; 12];
        let mut buffer = [0u8; 13];
        (&mut buffer[0..12]).copy_from_slice(&data);

        let encoded = hamming_encode(buffer);
        // {
        //     use bitvec::prelude::*;
        //     let buffer: BitArray<_, Lsb0> = BitArray::new(encoded.clone());

        //     for bit in buffer.iter() {
        //         print!("{}", if *bit { "1" } else { "0" });
        //     }
        //     println!("");
        // }

        for byte in 0..12 {
            for bit in 0..8 {
                let mut encoded = encoded.clone();
                encoded[byte] ^= 1 << bit;

                let decoded = hamming_decode(encoded).unwrap();

                assert_eq!(data, decoded, "byte {} bit {}", byte, bit);
            }
        }
    }
}
