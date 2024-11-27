# EarthEl Project

## Overview

The EarthEl project is a Rust-based application that retrieves elevation data for given latitude and longitude coordinates. It uses SRTM (Shuttle Radar Topography Mission) data stored in HGT files, which are downloaded and processed as needed.

## Features

- Fetches elevation data for specific geographic coordinates.
- Downloads and extracts HGT files from an S3 bucket.
- Supports both SRTM1 and SRTM3 resolutions.

## Dependencies

- `byteorder`
- `flate2`
- `reqwest`
- `tokio`

## Installation

1. Ensure you have Rust and Cargo installed. If not, you can install them from [here](https://www.rust-lang.org/tools/install).
2. Clone the repository:
   ```sh
   git clone <repository-url>
   cd <repository-directory>
   ```
3. Build the project:
   ```sh
   cargo build
   ```

## Usage

To use the EarthEl library, you can call the `get_elevation` function with the desired latitude and longitude:

```rust
use earth_el::EarthEl;

#[tokio::main]
async fn main() {
    let latitude = 47.0592;
    let longitude = 5.7181;
    match EarthEl::get_elevation(latitude, longitude).await {
        Ok(elevation) => println!("Elevation: {} meters", elevation),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## Testing

To run the tests, use the following command:

```sh
cargo test
```

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.

## Acknowledgements

- The SRTM data is provided by NASA.
- The project uses the `reqwest` library for HTTP requests and `flate2` for handling gzip files.