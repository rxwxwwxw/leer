use anyhow::Result;
use rusqlite::Connection;

#[derive(Clone, Debug)]
pub struct AbttTransformer {
    pub mean: Vec<f32>,
    pub principle: Vec<f32>,
}

impl AbttTransformer {
    pub fn from_db(conn: &Connection) -> Result<Self> {
        let mut stmt = conn.prepare("SELECT vector FROM model_transform_params WHERE key = ?1")?;
        let mean_blob: Vec<u8> = stmt.query_row(["mean_vector"], |r| r.get(0))?;
        let princ_blob: Vec<u8> = stmt.query_row(["principal_direction"], |r| r.get(0))?;

        let mean: Vec<f32> = mean_blob.chunks_exact(4).map(|b| f32::from_ne_bytes(b.try_into().unwrap())).collect();
        let principle: Vec<f32> = princ_blob.chunks_exact(4).map(|b| f32::from_ne_bytes(b.try_into().unwrap())).collect();

        Ok(Self { mean, principle })
    }

    pub fn transform(&self, mut vec: Vec<f32>) -> Vec<f32> {
        for (v, m) in vec.iter_mut().zip(self.mean.iter()) {
            *v -= *m;
        }
        let mut u = self.principle.clone();
        let u_norm = u.iter().map(|x| x * x).sum::<f32>().sqrt();
        if u_norm > 0.0 {
            for x in u.iter_mut() {
                *x /= u_norm;
            }
            let dot: f32 = vec.iter().zip(u.iter()).map(|(a, b)| a * b).sum();
            for (v, &u_i) in vec.iter_mut().zip(u.iter()) {
                *v -= dot * u_i;
            }
        }
        let final_norm = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if final_norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= final_norm;
            }
        }
        vec
    }
}
