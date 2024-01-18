use super::column::KeccakColumns;
use crate::DOMAIN_SIZE;
use ark_ff::{One, Zero};
use ark_poly::univariate::DensePolynomial;
use ark_poly::{Evaluations, Polynomial, Radix2EvaluationDomain as D};
use kimchi::groupmap::GroupMap;
use kimchi::{circuits::domains::EvaluationDomains, curve::KimchiCurve, plonk_sponge::FrSponge};
use mina_poseidon::sponge::ScalarChallenge;
use mina_poseidon::FqSponge;
use poly_commitment::evaluation_proof::DensePolynomialOrEvaluations;
use poly_commitment::OpenProof;
use poly_commitment::{
    commitment::{absorb_commitment, PolyComm},
    SRS as _,
};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

#[derive(Debug)]
pub struct KeccakProofInputs<G: KimchiCurve> {
    evaluations: KeccakColumns<Vec<G::ScalarField>>,
}

impl<G: KimchiCurve> Default for KeccakProofInputs<G> {
    fn default() -> Self {
        KeccakProofInputs {
            evaluations: KeccakColumns {
                hash_index: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                step_index: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flag_round: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flag_absorb: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flag_squeeze: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flag_root: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flag_pad: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flag_length: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                two_to_pad: (0..DOMAIN_SIZE).map(|_| G::ScalarField::one()).collect(),
                inverse_round: (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect(),
                flags_bytes: std::array::from_fn(|_| {
                    (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect()
                }),
                pad_suffix: std::array::from_fn(|_| {
                    (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect()
                }),
                round_constants: std::array::from_fn(|_| {
                    (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect()
                }),
                curr: std::array::from_fn(|_| {
                    (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect()
                }),
                next: std::array::from_fn(|_| {
                    (0..DOMAIN_SIZE).map(|_| G::ScalarField::zero()).collect()
                }),
            },
        }
    }
}

#[derive(Debug)]
pub struct KeccakProof<G: KimchiCurve, OpeningProof: OpenProof<G>> {
    _commitments: KeccakColumns<PolyComm<G>>,
    _zeta_evaluations: KeccakColumns<G::ScalarField>,
    _zeta_omega_evaluations: KeccakColumns<G::ScalarField>,
    _opening_proof: OpeningProof,
}

pub fn fold<
    G: KimchiCurve,
    OpeningProof: OpenProof<G>,
    EFqSponge: Clone + FqSponge<G::BaseField, G, G::ScalarField>,
    EFrSponge: FrSponge<G::ScalarField>,
>(
    domain: EvaluationDomains<G::ScalarField>,
    srs: &OpeningProof::SRS,
    accumulator: &mut KeccakProofInputs<G>,
    inputs: &KeccakColumns<Vec<G::ScalarField>>,
) where
    <OpeningProof as poly_commitment::OpenProof<G>>::SRS: std::marker::Sync,
{
    let commitments = {
        inputs
            .par_iter()
            .map(|evals: &Vec<G::ScalarField>| {
                let evals = Evaluations::<G::ScalarField, D<G::ScalarField>>::from_vec_and_domain(
                    evals.clone(),
                    domain.d1,
                );
                srs.commit_evaluations_non_hiding(domain.d1, &evals)
            })
            .collect::<KeccakColumns<_>>()
    };
    let mut fq_sponge = EFqSponge::new(G::other_curve_sponge_params());

    for column in commitments.into_iter() {
        absorb_commitment(&mut fq_sponge, &column);
    }
    let scaling_challenge = ScalarChallenge(fq_sponge.challenge());
    let (_, endo_r) = G::endos();
    let scaling_challenge = scaling_challenge.to_field(endo_r);
    accumulator
        .evaluations
        .par_iter_mut()
        .zip(inputs.par_iter())
        .for_each(|(accumulator, inputs)| {
            accumulator
                .par_iter_mut()
                .zip(inputs.par_iter())
                .for_each(|(accumulator, input)| {
                    *accumulator = *input + scaling_challenge * *accumulator
                });
        });
}

pub fn prove<
    G: KimchiCurve,
    OpeningProof: OpenProof<G>,
    EFqSponge: Clone + FqSponge<G::BaseField, G, G::ScalarField>,
    EFrSponge: FrSponge<G::ScalarField>,
>(
    domain: EvaluationDomains<G::ScalarField>,
    srs: &OpeningProof::SRS,
    inputs: KeccakProofInputs<G>,
) -> KeccakProof<G, OpeningProof>
where
    OpeningProof::SRS: Sync,
{
    let KeccakProofInputs { evaluations } = inputs;
    let polys = {
        let eval_col = |evals: Vec<G::ScalarField>| {
            Evaluations::<G::ScalarField, D<G::ScalarField>>::from_vec_and_domain(evals, domain.d1)
                .interpolate()
        };
        let eval_array_col = |evals: &[Vec<G::ScalarField>]| {
            evals
                .into_par_iter()
                .map(|e| eval_col(e.to_vec()))
                .collect::<Vec<_>>()
        };
        KeccakColumns {
            hash_index: eval_col(evaluations.hash_index),
            step_index: eval_col(evaluations.step_index),
            flag_round: eval_col(evaluations.flag_round),
            flag_absorb: eval_col(evaluations.flag_absorb),
            flag_squeeze: eval_col(evaluations.flag_squeeze),
            flag_root: eval_col(evaluations.flag_root),
            flag_pad: eval_col(evaluations.flag_pad),
            flag_length: eval_col(evaluations.flag_length),
            two_to_pad: eval_col(evaluations.two_to_pad),
            inverse_round: eval_col(evaluations.inverse_round),
            flags_bytes: eval_array_col(&evaluations.flags_bytes).try_into().unwrap(),
            pad_suffix: eval_array_col(&evaluations.pad_suffix).try_into().unwrap(),
            round_constants: eval_array_col(&evaluations.round_constants)
                .try_into()
                .unwrap(),
            curr: eval_array_col(&evaluations.curr).try_into().unwrap(),
            next: eval_array_col(&evaluations.next).try_into().unwrap(),
        }
    };
    let commitments = {
        let comm = |poly: &DensePolynomial<G::ScalarField>| srs.commit_non_hiding(poly, 1, None);
        let comm_array = |polys: &[DensePolynomial<G::ScalarField>]| {
            polys.into_par_iter().map(comm).collect::<Vec<_>>()
        };
        KeccakColumns {
            hash_index: comm(&polys.hash_index),
            step_index: comm(&polys.step_index),
            flag_round: comm(&polys.flag_round),
            flag_absorb: comm(&polys.flag_absorb),
            flag_squeeze: comm(&polys.flag_squeeze),
            flag_root: comm(&polys.flag_root),
            flag_pad: comm(&polys.flag_pad),
            flag_length: comm(&polys.flag_length),
            two_to_pad: comm(&polys.two_to_pad),
            inverse_round: comm(&polys.inverse_round),
            flags_bytes: comm_array(&polys.flags_bytes).try_into().unwrap(),
            pad_suffix: comm_array(&polys.pad_suffix).try_into().unwrap(),
            round_constants: comm_array(&polys.round_constants).try_into().unwrap(),
            curr: comm_array(&polys.curr).try_into().unwrap(),
            next: comm_array(&polys.next).try_into().unwrap(),
        }
    };

    let mut fq_sponge = EFqSponge::new(G::other_curve_sponge_params());

    for column in commitments.clone().into_iter() {
        absorb_commitment(&mut fq_sponge, &column);
    }
    let zeta_chal = ScalarChallenge(fq_sponge.challenge());
    let (_, endo_r) = G::endos();
    let zeta = zeta_chal.to_field(endo_r);
    let omega = domain.d1.group_gen;
    let zeta_omega = zeta * omega;

    let evals = |point| {
        let comm = |poly: &DensePolynomial<G::ScalarField>| poly.evaluate(point);
        let comm_array = |polys: &[DensePolynomial<G::ScalarField>]| {
            polys.par_iter().map(comm).collect::<Vec<_>>()
        };
        KeccakColumns {
            hash_index: comm(&polys.hash_index),
            step_index: comm(&polys.step_index),
            flag_round: comm(&polys.flag_round),
            flag_absorb: comm(&polys.flag_absorb),
            flag_squeeze: comm(&polys.flag_squeeze),
            flag_root: comm(&polys.flag_root),
            flag_pad: comm(&polys.flag_pad),
            flag_length: comm(&polys.flag_length),
            two_to_pad: comm(&polys.two_to_pad),
            inverse_round: comm(&polys.inverse_round),
            flags_bytes: comm_array(&polys.flags_bytes).try_into().unwrap(),
            pad_suffix: comm_array(&polys.pad_suffix).try_into().unwrap(),
            round_constants: comm_array(&polys.round_constants).try_into().unwrap(),
            curr: comm_array(&polys.curr).try_into().unwrap(),
            next: comm_array(&polys.next).try_into().unwrap(),
        }
    };
    let zeta_evaluations = evals(&zeta);
    let zeta_omega_evaluations = evals(&zeta_omega);
    let group_map = G::Map::setup();
    let polynomials = polys.into_iter().collect::<Vec<_>>();
    let polynomials: Vec<_> = polynomials
        .iter()
        .map(|poly| {
            (
                DensePolynomialOrEvaluations::DensePolynomial(poly),
                None,
                PolyComm {
                    unshifted: vec![G::ScalarField::zero()],
                    shifted: None,
                },
            )
        })
        .collect();
    let fq_sponge_before_evaluations = fq_sponge.clone();
    let mut fr_sponge = EFrSponge::new(G::sponge_params());
    fr_sponge.absorb(&fq_sponge.digest());

    for (zeta_eval, zeta_omega_eval) in zeta_evaluations
        .clone()
        .into_iter()
        .zip(zeta_omega_evaluations.clone().into_iter())
    {
        fr_sponge.absorb(&zeta_eval);
        fr_sponge.absorb(&zeta_omega_eval);
    }

    let v_chal = fr_sponge.challenge();
    let v = v_chal.to_field(endo_r);
    let u_chal = fr_sponge.challenge();
    let u = u_chal.to_field(endo_r);

    let opening_proof = OpenProof::open::<_, _, D<G::ScalarField>>(
        srs,
        &group_map,
        polynomials.as_slice(),
        &[zeta, zeta_omega],
        v,
        u,
        fq_sponge_before_evaluations,
        &mut rand::rngs::OsRng,
    );

    KeccakProof {
        _commitments: commitments,
        _zeta_evaluations: zeta_evaluations,
        _zeta_omega_evaluations: zeta_omega_evaluations,
        _opening_proof: opening_proof,
    }
}
