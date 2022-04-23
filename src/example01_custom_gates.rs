use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::{
    circuit::{Layouter, Region, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};

use std::marker::PhantomData;

#[derive(Clone)]
struct EqOneConfig<F> {
    q_enable: Selector,
    a: Column<Advice>,
    _marker: PhantomData<F>,
}
impl<F: FieldExt> EqOneConfig<F> {
    fn configure(meta: &mut ConstraintSystem<F>, q_enable: Selector, a: Column<Advice>) -> Self {
        meta.create_gate("a is one", |meta| {
            let q = meta.query_selector(q_enable);
            let a = meta.query_advice(a, Rotation::cur());
            let one = Expression::Constant(F::one());
            vec![("check a", q * (a - one))]
        });
        Self {
            q_enable,
            a,
            _marker: PhantomData,
        }
    }

    fn assign(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        dummy: Option<F>,
    ) -> Result<(), Error> {
        self.q_enable.enable(region, offset)?;
        region.assign_advice(|| "a", self.a, offset, || dummy.ok_or(Error::Synthesis))?;
        Ok(())
    }
}

#[derive(Default)]
struct MyCircuit<F> {
    a: Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = EqOneConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let a = meta.advice_column();
        let q_enable = meta.selector();
        EqOneConfig::configure(meta, q_enable, a)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "assign a",
            |mut region| {
                let offset = 0;
                config.assign(&mut region, offset, self.a)?;
                Ok(())
            },
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn test_successful_case() {
        let circuit = MyCircuit::<Fp> { a: Some(Fp::one()) };
        let k = 3;
        let prover = MockProver::<Fp>::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));

        #[cfg(feature = "dev-graph")]
        {
            use plotters::prelude::*;
            let root = BitMapBackend::new("example01.png", (1024, 768)).into_drawing_area();
            root.fill(&WHITE).unwrap();
            let root = root.titled("a is one", ("sans-serif", 60)).unwrap();

            halo2_proofs::dev::CircuitLayout::default()
                .view_width(0..2)
                .view_height(0..16)
                .show_labels(true)
                .render(k, &circuit, &root)
                .unwrap();
        }
    }
}
