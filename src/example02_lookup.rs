use halo2_gadgets::poseidon::primitives::{Hash, P128Pow5T3};
use halo2_proofs::pasta::Fp;
use halo2_proofs::{
    circuit::{Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector, TableColumn},
    poly::Rotation,
};

#[derive(Clone)]
struct PoseidonConfig {
    q_enable: Selector,
    a: Column<Advice>,
    poseidon: Column<Advice>,
    table: PoseidonTableConfig,
}

impl PoseidonConfig {
    pub fn configure(meta: &mut ConstraintSystem<Fp>) -> Self {
        let q_enable = meta.complex_selector();
        let a = meta.advice_column();
        let poseidon = meta.advice_column();

        let table = PoseidonTableConfig {
            a: meta.lookup_table_column(),
            poseidon: meta.lookup_table_column(),
        };

        meta.lookup(|meta| {
            let q_enable = meta.query_selector(q_enable);
            let a = meta.query_advice(a, Rotation::cur());
            let poseidon = meta.query_advice(poseidon, Rotation::cur());

            let not_q_enable = Expression::Constant(Fp::one()) - q_enable.clone();
            let default_a = Expression::Constant(Fp::zero());
            let hasher = Hash::<_, P128Pow5T3, _, 3, 2>::init();
            let default_poseidon = hasher.hash([Fp::zero()]);

            vec![
                (
                    q_enable.clone() * a + not_q_enable.clone() * default_a,
                    table.a,
                ),
                (
                    q_enable * poseidon + not_q_enable * default_poseidon,
                    table.poseidon,
                ),
            ]
        });
        Self {
            q_enable,
            a,
            poseidon,
            table,
        }
    }

    pub fn assign(
        &self,
        region: &mut Region<'_, Fp>,
        offset: usize,
        a: Value<Fp>,
    ) -> Result<(), Error> {
        self.q_enable.enable(region, offset)?;
        region.assign_advice(|| "a", self.a, offset, || a)?;

        let poseidon = a.map(|a| {
            let hasher = Hash::<_, P128Pow5T3, _, 3, 2>::init();
            hasher.hash([a])
        });

        region.assign_advice(|| "poseidon", self.poseidon, offset, || poseidon)?;

        Ok(())
    }
}

#[derive(Clone)]
struct PoseidonTableConfig {
    a: TableColumn,
    poseidon: TableColumn,
}

impl PoseidonTableConfig {
    pub fn load(&self, layouter: &mut impl Layouter<Fp>) -> Result<(), Error> {
        layouter.assign_table(
            || "Poseidon table",
            |mut table| {
                for (offset, a) in (0..(1 << 4)).enumerate() {
                    let a = Value::known(Fp::from(a as u64));
                    table.assign_cell(|| "a", self.a, offset, || a)?;

                    let hasher = Hash::<_, P128Pow5T3, _, 3, 2>::init();
                    let hash = a.map(|a| hasher.hash([a]));
                    table.assign_cell(|| "Poseidon", self.poseidon, offset, || hash)?;
                }

                Ok(())
            },
        )
    }
}

#[derive(Default)]
struct MyCircuit {
    a: Value<Fp>,
}

impl Circuit<Fp> for MyCircuit {
    type Config = PoseidonConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        PoseidonConfig::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fp>,
    ) -> Result<(), Error> {
        config.table.load(&mut layouter)?;
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
        let circuit = MyCircuit {
            a: Value::known(Fp::one()),
        };
        let k = 5;
        let prover = MockProver::<Fp>::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
        // assert_eq!(prover.verify(), Ok(()));

        #[cfg(feature = "dev-graph")]
        {
            use plotters::prelude::*;
            let root = BitMapBackend::new("example02.png", (500, 800)).into_drawing_area();
            root.fill(&WHITE).unwrap();
            let root = root.titled("Poseidon lookup", ("sans-serif", 60)).unwrap();

            halo2_proofs::dev::CircuitLayout::default()
                .view_width(0..4)
                .view_height(0..32)
                .show_labels(true)
                .render(k, &circuit, &root)
                .unwrap();
        }
    }
}
