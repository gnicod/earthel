use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::GzDecoder;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::PathBuf;

pub struct EarthEl;

struct HgtFile {
    folder: String,
    name: String,
    path: PathBuf,
}


impl HgtFile {
    async fn new(latitude : f64, longitude: f64) -> Self {
        let lat_prefix = if latitude >= 0.0 { "N" } else { "S" };
        let lon_prefix = if longitude >= 0.0 { "E" } else { "W" };
        let lat_int = latitude.abs().floor() as i32;
        let lon_int = longitude.abs().floor() as i32;
        let name = format!("{}{:02}{}{:03}.hgt", lat_prefix, lat_int, lon_prefix, lon_int);
        let folder = format!("{}{}", lat_prefix, lat_int);
        let path = PathBuf::from(format!("/tmp/hgt/{}/{}",folder, name));
        Self{
            folder,
            name,
            path,
        }
    }

    async fn get_file(&self) -> std::io::Result<File> {
        if !self.path.exists() {
            self.download_hgt().await.expect("Error downloading file");
        }
        File::open(&self.path)
    }

    async fn download_hgt(&self) -> Result<()> {
        let path_s3 = format!("https://elevation-tiles-prod.s3.amazonaws.com/skadi/{}/{}.gz", self.folder, self.name);
        let response = reqwest::get(path_s3).await?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = PathBuf::from("/tmp/tmp_hgt.hgt.gz");
        let mut file = std::fs::File::create(tmp_path)?;
        let mut content =  Cursor::new(response.bytes().await?);
        std::io::copy(&mut content, &mut file)?;

        self.extract_gz_file().expect("TODO: panic message");

        Ok(())
    }

    fn extract_gz_file(&self) -> std::io::Result<()> {
        let input_path = PathBuf::from("/tmp/tmp_hgt.hgt.gz");
        let gz_file = File::open(input_path)?;
        let mut decoder = GzDecoder::new(gz_file);
        let mut output_file = File::create(&self.path)?;
        std::io::copy(&mut decoder, &mut output_file)?;
        Ok(())
    }

    pub fn get_resolution(&self) -> Option<usize> {
        let from_metadata = |m: fs::Metadata| match m.len() {
            25934402 => Some(3601), // SRTM1
            2884802 => Some(1201),  // SRTM3
            _ => Some(3601),        // Default to SRTM3
        };
        fs::metadata(&self.path).ok().and_then(from_metadata)
    }
}


impl EarthEl {
    pub async fn get_elevation(latitude: f64, longitude : f64) -> Result<i16>{
        let hgt_file = HgtFile::new(latitude, longitude).await;
        let mut file= hgt_file.get_file().await.expect("file not found");
        let grid_size: usize = hgt_file.get_resolution().expect("failed to get resolution");
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

type Result<PathBuf> = std::result::Result<PathBuf, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Display;


    #[tokio::test]
    async fn it_works() {
        let el = EarthEl::get_elevation(47.0592,5.7181).await.expect("error");
        assert_eq!( el, 259 );
        let el = EarthEl::get_elevation(45.833641, 6.864594).await.expect("error");
        assert_eq!( el, 4740 );
        print!("Mont blanc {el}")
    }
}