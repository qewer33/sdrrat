use futuresdr::num_complex::Complex32;
use futuresdr::runtime::dev::prelude::*;

/// 1-in 2-out fanout block for `Complex32`.
///
/// FutureSDR streams have single-reader buffers, so to feed two downstream
/// chains (FFT path + demod path) from one source we need an explicit copy.
#[derive(Block)]
pub struct Tee<
    I: CpuBufferReader<Item = Complex32> = DefaultCpuReader<Complex32>,
    O1: CpuBufferWriter<Item = Complex32> = DefaultCpuWriter<Complex32>,
    O2: CpuBufferWriter<Item = Complex32> = DefaultCpuWriter<Complex32>,
> {
    #[input]
    input: I,
    #[output]
    out_a: O1,
    #[output]
    out_b: O2,
}

impl<I, O1, O2> Tee<I, O1, O2>
where
    I: CpuBufferReader<Item = Complex32>,
    O1: CpuBufferWriter<Item = Complex32>,
    O2: CpuBufferWriter<Item = Complex32>,
{
    pub fn new() -> Self {
        Self {
            input: I::default(),
            out_a: O1::default(),
            out_b: O2::default(),
        }
    }
}

#[doc(hidden)]
impl<I, O1, O2> Kernel for Tee<I, O1, O2>
where
    I: CpuBufferReader<Item = Complex32>,
    O1: CpuBufferWriter<Item = Complex32>,
    O2: CpuBufferWriter<Item = Complex32>,
{
    async fn work(
        &mut self,
        io: &mut WorkIo,
        _mo: &mut MessageOutputs,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        let i_len = self.input().slice().len();
        let oa_len = self.out_a().slice().len();
        let ob_len = self.out_b().slice().len();
        let m = i_len.min(oa_len).min(ob_len);

        if m > 0 {
            // Snapshot input to a local Vec so we can release that borrow
            // before mutating the two output slices.
            let snapshot: Vec<Complex32> = self.input().slice()[..m].to_vec();
            self.out_a().slice()[..m].copy_from_slice(&snapshot);
            self.out_b().slice()[..m].copy_from_slice(&snapshot);
            self.input().consume(m);
            self.out_a().produce(m);
            self.out_b().produce(m);
        }

        if self.input().finished() && m == i_len {
            io.finished = true;
        }

        Ok(())
    }
}
