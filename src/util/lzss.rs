const WINDOW_SIZE: usize = 0x10000;
const WINDOW_BASE: usize = 0xFEFD;

pub fn decompress(src: &[u8], dst_len: usize) -> Vec<u8> {
    let src_len = src.len();
    let mut dst = vec![0; dst_len];
    let mut win = [0; WINDOW_SIZE];
    let mut win_pos = WINDOW_BASE;
    let mut dst_pos = 0;
    let mut src_pos = 0;

    while dst_pos < dst_len && src_pos < src_len {
        let flags = src[src_pos];
        src_pos += 1;

        for shift in 0..8 {
            // if flags & (1 << shift) != 0 {
            if (flags >> shift) & 1 == 1 {
                let byte = src[src_pos];

                win[win_pos] = byte;
                dst[dst_pos] = byte;

                dst_pos += 1;
                src_pos += 1;
                win_pos = (win_pos + 1) % WINDOW_SIZE;
            } else {
                if src_pos > src_len - 3 {
                    return dst;
                }

                let mut offset = u16::from_le_bytes([src[src_pos], src[src_pos + 1]]) as usize;
                let length = 4 + src[src_pos + 2] as usize;
                src_pos += 3;

                for _ in 0..length {
                    if dst_pos >= dst_len {
                        break;
                    }

                    let byte = win[offset];

                    win[win_pos] = byte;
                    dst[dst_pos] = byte;

                    offset = (offset + 1) % WINDOW_SIZE;
                    win_pos = (win_pos + 1) % WINDOW_SIZE;
                    dst_pos += 1;
                }
            }
        }
    }

    dst
}
