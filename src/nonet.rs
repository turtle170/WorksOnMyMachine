//! NoNet: an 8 -> 32 -> 16 -> 1 multilayer perceptron with exactly 833
//! parameters (288 + 528 + 17). 3.33 KB as f32, 833 bytes if you squint.
//!
//! The compiler quirk: before doing anything, womm constructs NoNet, which does
//! nothing at first. AdamW then tunes it on a fixed slice of the womm binary's
//! own bytes (predict the next byte from the previous eight). The trained net
//! then turns the big hash into an extra hash, and picks every word womm says.
//!
//! The activation is a rational tanh built only from + - * / and a clamp. That
//! keeps NoNet bit-for-bit reproducible anywhere IEEE-754 f32 exists, including
//! in the Python that `womm visualise` writes.

use crate::rng::{Chooser, Rng};

pub const IN: usize = 8;
pub const H1: usize = 32;
pub const H2: usize = 16;
pub const PARAMS: usize = IN * H1 + H1 + H1 * H2 + H2 + H2 + 1;

pub const W1: usize = 0;
pub const B1: usize = W1 + IN * H1;
pub const W2: usize = B1 + H1;
pub const B2: usize = W2 + H1 * H2;
pub const W3: usize = B2 + H2;
pub const B3: usize = W3 + H2;

const TRAIN_STEPS: usize = 200;
const BATCH: usize = 32;
const SLICE: usize = 4096;
const LR: f32 = 0.01;
const WEIGHT_DECAY: f32 = 0.01;
const BETA1: f32 = 0.9;
const BETA2: f32 = 0.999;
const EPS: f32 = 1e-8;

pub const EXTRA_MIN: usize = 16;
pub const WINDOWS: usize = 128;

/// Rational tanh: x(27 + x^2) / (27 + 9x^2), clamped to [-3, 3] (where it hits +-1).
pub fn act(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    let t1 = 27.0 + x2;
    let num = x * t1;
    let t2 = 9.0 * x2;
    let den = 27.0 + t2;
    num / den
}

fn act_grad(pre: f32) -> f32 {
    if !(-3.0..=3.0).contains(&pre) {
        return 0.0;
    }
    let x2 = pre * pre;
    let den = 27.0 + 9.0 * x2;
    ((27.0 + 3.0 * x2) * den - pre * (27.0 + x2) * (18.0 * pre)) / (den * den)
}

/// Bytes become inputs in [-0.5, 0.5].
pub fn input(bytes: &[u8]) -> [f32; IN] {
    let mut x = [0f32; IN];
    for (i, b) in bytes.iter().take(IN).enumerate() {
        x[i] = *b as f32 / 255.0 - 0.5;
    }
    x
}

struct Cache {
    pre1: [f32; H1],
    h1: [f32; H1],
    pre2: [f32; H2],
    h2: [f32; H2],
    y: f32,
}

#[derive(Clone)]
pub struct Nonet {
    pub p: Vec<f32>,
}

impl Nonet {
    /// The freshly constructed net. It does nothing yet.
    pub fn untrained(seed: &[u8]) -> Nonet {
        let mut rng = Rng::from_seed(&[b"nonet-init:".as_slice(), seed].concat());
        let mut p = vec![0f32; PARAMS];
        let mut fill = |range: std::ops::Range<usize>, fan_in: usize| {
            let scale = 1.0 / (fan_in as f32).sqrt();
            for w in &mut p[range] {
                *w = (rng.unit() * 2.0 - 1.0) * scale;
            }
        };
        fill(W1..B1, IN);
        fill(W2..B2, H1);
        fill(W3..B3, H2);
        Nonet { p }
    }

    fn forward_cache(&self, x: &[f32; IN]) -> Cache {
        let p = &self.p;
        let mut c = Cache { pre1: [0.0; H1], h1: [0.0; H1], pre2: [0.0; H2], h2: [0.0; H2], y: 0.0 };
        for j in 0..H1 {
            let mut acc = p[B1 + j];
            for (i, xi) in x.iter().enumerate() {
                acc += xi * p[W1 + i * H1 + j];
            }
            c.pre1[j] = acc;
            c.h1[j] = act(acc);
        }
        for k in 0..H2 {
            let mut acc = p[B2 + k];
            for j in 0..H1 {
                acc += c.h1[j] * p[W2 + j * H2 + k];
            }
            c.pre2[k] = acc;
            c.h2[k] = act(acc);
        }
        let mut y = p[B3];
        for k in 0..H2 {
            y += c.h2[k] * p[W3 + k];
        }
        c.y = y;
        c
    }

    pub fn forward(&self, x: &[f32; IN]) -> f32 {
        self.forward_cache(x).y
    }

    /// AdamW on a fixed slice of `data`. Returns the last batch's loss.
    pub fn train(&mut self, data: &[u8], seed: &[u8], steps: usize) -> f32 {
        let start = (data.len() / 3).min(data.len().saturating_sub(SLICE));
        let slice = &data[start..(start + SLICE).min(data.len())];
        if slice.len() <= IN {
            return 0.0;
        }
        let mut rng = Rng::from_seed(&[b"nonet-data:".as_slice(), seed].concat());
        let (mut m, mut v) = (vec![0f32; PARAMS], vec![0f32; PARAMS]);
        let (mut b1t, mut b2t) = (1f32, 1f32);
        let mut loss = 0.0;

        for _ in 0..steps {
            let mut g = vec![0f32; PARAMS];
            loss = 0.0;
            for _ in 0..BATCH {
                let pos = rng.below(slice.len() - IN);
                let x = input(&slice[pos..pos + IN]);
                let target = slice[pos + IN] as f32 / 255.0 - 0.5;
                let c = self.forward_cache(&x);
                let err = c.y - target;
                loss += err * err / BATCH as f32;
                let dy = 2.0 * err / BATCH as f32;

                g[B3] += dy;
                let mut dh2 = [0f32; H2];
                for k in 0..H2 {
                    g[W3 + k] += dy * c.h2[k];
                    dh2[k] = dy * self.p[W3 + k] * act_grad(c.pre2[k]);
                }
                let mut dh1 = [0f32; H1];
                for k in 0..H2 {
                    g[B2 + k] += dh2[k];
                    for j in 0..H1 {
                        g[W2 + j * H2 + k] += dh2[k] * c.h1[j];
                        dh1[j] += dh2[k] * self.p[W2 + j * H2 + k];
                    }
                }
                for j in 0..H1 {
                    let d = dh1[j] * act_grad(c.pre1[j]);
                    g[B1 + j] += d;
                    for i in 0..IN {
                        g[W1 + i * H1 + j] += d * x[i];
                    }
                }
            }
            b1t *= BETA1;
            b2t *= BETA2;
            for i in 0..PARAMS {
                m[i] = BETA1 * m[i] + (1.0 - BETA1) * g[i];
                v[i] = BETA2 * v[i] + (1.0 - BETA2) * g[i] * g[i];
                let mh = m[i] / (1.0 - b1t);
                let vh = v[i] / (1.0 - b2t);
                self.p[i] -= LR * (mh / (vh.sqrt() + EPS) + WEIGHT_DECAY * self.p[i]);
            }
        }
        loss
    }

    /// Mean squared error on a fixed set of windows from the same slice
    /// training uses. Honest, unlike most of womm.
    pub fn eval(&self, data: &[u8]) -> f32 {
        let start = (data.len() / 3).min(data.len().saturating_sub(SLICE));
        let slice = &data[start..(start + SLICE).min(data.len())];
        if slice.len() <= IN {
            return 0.0;
        }
        let n = 256.min(slice.len() - IN);
        let step = (slice.len() - IN) / n;
        (0..n)
            .map(|i| {
                let pos = i * step;
                let err = self.forward(&input(&slice[pos..pos + IN])) - (slice[pos + IN] as f32 / 255.0 - 0.5);
                err * err
            })
            .sum::<f32>()
            / n as f32
    }

    /// Slide an 8-byte window over the big hash; each window yields one byte.
    /// Stops when NoNet says so, which is why the result has a variable length.
    pub fn extra_hash(&self, big: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for (w, window) in big.as_chunks::<IN>().0.iter().take(WINDOWS).enumerate() {
            let y = self.forward(&input(window));
            let v = (y * 65536.0) as i64;
            out.push((v & 0xff) as u8 ^ window[w % IN]);
            if out.len() >= EXTRA_MIN && (v >> 8) & 0x1f == 0 {
                break;
            }
        }
        out
    }
}

/// NoNet as the compiler builds it: constructed from `seed`, then trained on
/// the womm binary itself.
pub fn compiler_nonet(seed: &[u8]) -> Nonet {
    let mut net = Nonet::untrained(seed);
    let binary = crate::payload::base_stub().unwrap_or_default();
    net.train(&binary, seed, TRAIN_STEPS);
    net
}

/// NoNet as a speaker: every choice womm makes about what to say goes through it.
pub struct Voice {
    pub net: Nonet,
    rng: Rng,
}

impl Voice {
    pub fn new(seed: &[u8]) -> Voice {
        Voice::with_net(compiler_nonet(seed), seed)
    }

    pub fn with_net(net: Nonet, seed: &[u8]) -> Voice {
        Voice { net, rng: Rng::from_seed(&[b"voice:".as_slice(), seed].concat()) }
    }

    /// One raw NoNet opinion about a fresh random input, in roughly [-1, 1].
    pub fn opinion(&mut self) -> f32 {
        let x = input(&self.rng.bytes8());
        self.net.forward(&x)
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }
}

impl Chooser for Voice {
    fn pick(&mut self, n: usize) -> usize {
        let v = (self.opinion() * 65536.0) as i64 as u64;
        ((self.rng.next_u64() ^ v) % n.max(1) as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_exactly_833_parameters() {
        assert_eq!(PARAMS, 833);
        assert_eq!(IN * H1 + H1, 288);
        assert_eq!(H1 * H2 + H2, 528);
        assert_eq!(H2 + 1, 17);
        assert_eq!(B3, 832);
    }

    #[test]
    fn act_is_bounded_and_hits_one() {
        assert_eq!(act(3.0), 1.0);
        assert_eq!(act(-10.0), -1.0);
        assert_eq!(act(0.0), 0.0);
    }

    #[test]
    fn training_is_deterministic_and_reduces_loss() {
        let data: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let eval = |net: &Nonet| -> f32 {
            (0..200)
                .map(|i| {
                    let pos = 1700 + i * 9;
                    let y = net.forward(&input(&data[pos..pos + IN]));
                    let t = data[pos + IN] as f32 / 255.0 - 0.5;
                    (y - t) * (y - t)
                })
                .sum::<f32>()
                / 200.0
        };
        let mut a = Nonet::untrained(b"s");
        let mut b = Nonet::untrained(b"s");
        let before = eval(&a);
        a.train(&data, b"s", 200);
        b.train(&data, b"s", 200);
        assert_eq!(a.p, b.p);
        let after = eval(&a);
        assert!(after < before, "eval loss {after} should drop below {before}");
    }

    #[test]
    fn extra_hash_is_variable_but_stable() {
        let net = Nonet::untrained(b"x");
        let big: Vec<u8> = (0..1024u32).map(|i| (i * 31 % 256) as u8).collect();
        let e = net.extra_hash(&big);
        assert!(e.len() >= EXTRA_MIN && e.len() <= WINDOWS);
        assert_eq!(e, net.extra_hash(&big));
    }
}
