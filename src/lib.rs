use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::GzDecoder;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::PathBuf;
use thiserror::Error;

pub struct EarthEl;

#[derive(Debug, Error)]
pub enum HgtError {
    #[error("File operation failed: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to decode gzip file: {0}")]
    DecodeError(#[from] flate2::DecompressError),

    #[error("Network error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Unexpected HGT resolution: {0}")]
    InvalidResolution(u64),
}

struct HgtFile {
    folder: String,
    name: String,
    path: PathBuf,
}

impl HgtFile {
    async fn new(latitude: f64, longitude: f64) -> Self {
        let lat_prefix = if latitude >= 0.0 { "N" } else { "S" };
        let lon_prefix = if longitude >= 0.0 { "E" } else { "W" };
        let lat_int = latitude.abs().floor() as i32;
        let lon_int = longitude.abs().floor() as i32;
        let name = format!(
            "{}{:02}{}{:03}.hgt",
            lat_prefix, lat_int, lon_prefix, lon_int
        );
        let folder = format!("{}{}", lat_prefix, lat_int);
        let path = PathBuf::from(format!("/tmp/hgt/{}/{}", folder, name));
        Self { folder, name, path }
    }

    async fn get_file(&self) -> std::result::Result<File, HgtError> {
        if !self.path.exists() {
            self.download_hgt().await?;
        }
        File::open(&self.path).map_err(HgtError::from)
    }

    async fn download_hgt(&self) -> Result<()> {
        let path_s3 = format!(
            "https://elevation-tiles-prod.s3.amazonaws.com/skadi/{}/{}.gz",
            self.folder, self.name
        );
        let response = reqwest::get(&path_s3).await?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(HgtError::from)?;
        }
        let tmp_path = PathBuf::from(format!("/tmp/tmp_{}.gz", self.name));
        let mut file = File::create(tmp_path)?;
        let mut content = Cursor::new(response.bytes().await?);
        std::io::copy(&mut content, &mut file)?;
        self.extract_gz_file()?;
        Ok(())
    }

    fn extract_gz_file(&self) -> std::io::Result<()> {
        let input_path = PathBuf::from(format!("/tmp/tmp_{}.gz", self.name));
        let gz_file = File::open(&input_path)?;
        let mut decoder = GzDecoder::new(gz_file);
        let mut output_file = File::create(&self.path)?;
        std::io::copy(&mut decoder, &mut output_file)?;
        fs::remove_file(input_path)?;
        Ok(())
    }

    pub fn get_resolution(&self) -> Option<usize> {
        let from_metadata = |m: fs::Metadata| match m.len() {
            25934402 => Some(3601), // SRTM1
            2884802 => Some(1201),  // SRTM3
            _ => None,              // Default to SRTM3
        };
        fs::metadata(&self.path).ok().and_then(from_metadata)
    }
}

impl EarthEl {
    // Generate doc string
    /// Retrieves the elevation data for the given latitude and longitude coordinates.
    ///
    /// This function downloads the necessary HGT file if it is not already available locally,
    /// extracts the elevation data from the file, and returns the elevation in meters.
    ///
    /// # Arguments
    ///
    /// * `latitude` - A f64 representing the latitude of the location.
    /// * `longitude` - A f64 representing the longitude of the location.
    ///
    /// # Returns
    ///
    /// A `Result` containing the elevation in meters as an i16, or an error if the operation fails.
    ///
    /// # Example
    ///
    /// ```
    /// use earth_el::EarthEl;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let latitude = 47.0592;
    ///     let longitude = 5.7181;
    ///     match EarthEl::get_elevation(latitude, longitude).await {
    ///         Ok(elevation) => println!("Elevation: {} meters", elevation),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn get_elevation(latitude: f64, longitude: f64) -> Result<i16> {
        let hgt_file = HgtFile::new(latitude, longitude).await;
        let mut file = hgt_file.get_file().await?;
        let grid_size: usize = hgt_file
            .get_resolution()
            .ok_or_else(|| HgtError::InvalidResolution(0))?;
        let lat_seconds = ((latitude - latitude.floor()) * 3600.0) as usize;
        let lon_seconds = ((longitude - longitude.floor()) * 3600.0) as usize;
        let lat_pos = (grid_size - 1) - (lat_seconds * (grid_size - 1) / 3600);
        let lon_pos = lon_seconds * (grid_size - 1) / 3600;
        let pos = 2 * (lat_pos * grid_size + lon_pos);
        file.seek(SeekFrom::Start(pos as u64))?;
        let elevation = file.read_i16::<BigEndian>()?;
        Ok(elevation)
    }
}

type Result<T> = std::result::Result<T, HgtError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let el = EarthEl::get_elevation(47.0592, 5.7181)
            .await
            .expect("error");
        assert_eq!(el, 259);
        let el = EarthEl::get_elevation(45.833641, 6.864594)
            .await
            .expect("error");
        assert_eq!(el, 4740);
        print!("Mont blanc {el}")
    }
}
