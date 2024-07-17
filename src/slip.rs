use core::convert::Infallible;

use embedded_io_async::{ErrorType, Read};

const END: u8 = 192;
const ESC: u8 = 219;
const ESC_END: u8 = 220;
const ESC_ESC: u8 = 221;

pub fn encode_slip(input: &[u8], output: &mut [u8]) -> usize {
    let mut offset = 0usize;

    for i in 0..input.len() {
        let byte = input[i];
        if byte == END {
            output[offset] = ESC;
            offset += 1;
            output[offset] = ESC_END;
            offset += 1;
        } else if byte == ESC {
            output[offset] = ESC;
            offset += 1;
            output[offset] = ESC_ESC;
            offset += 1;
        } else {
            output[offset] = byte;
            offset += 1;
        }
    }
    output[offset] = END;
    offset += 1;

    offset
}

pub struct SLIPDecoder<Reader: Read, const MAX_LEN: usize> {
    reader: Reader,
    buf: [u8; MAX_LEN],
    buf_index: usize,
}

impl<Reader: Read, const MAX_LEN: usize> SLIPDecoder<Reader, MAX_LEN> {
    pub fn new(reader: Reader) -> Self {
        Self {
            reader,
            buf: [0; MAX_LEN],
            buf_index: 0,
        }
    }

    pub fn read_sync(&mut self, res: &mut [u8]) -> usize {
        let mut esc = false;
        let mut ended = false;
        let mut read_amount = 0;
        let mut write_amount = 0;
        let mut copy_i = 0;
        for i in 0..self.buf_index {
            let byte = self.buf[i];
            if ended {
                self.buf[copy_i] = self.buf[i];
                copy_i += 1;
            } else {
                read_amount += 1;
                if byte == END {
                    ended = true;
                } else if byte == ESC {
                    esc = true;
                } else {
                    if esc {
                        esc = false;
                        if byte == ESC_END {
                            res[write_amount] = END;
                            write_amount += 1;
                        } else if byte == ESC_ESC {
                            res[write_amount] = ESC;
                            write_amount += 1;
                        }
                    } else {
                        res[write_amount] = byte;
                        write_amount += 1;
                    }
                }
            }
        }

        if ended {
            self.buf_index -= read_amount;
            write_amount
        } else {
            0
        }
    }
}

impl<Reader: Read, const MAX_LEN: usize> ErrorType for SLIPDecoder<Reader, MAX_LEN> {
    type Error = Infallible;
}

impl<Reader: Read, const MAX_LEN: usize> Read for SLIPDecoder<Reader, MAX_LEN> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.buf_index += self
            .reader
            .read(&mut self.buf[self.buf_index..MAX_LEN])
            .await
            .unwrap();

        Ok(self.read_sync(buf))
    }
}
