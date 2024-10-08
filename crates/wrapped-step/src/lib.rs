use flood_rs::prelude::*;
use std::io;

#[derive(Debug)]
pub struct GenericOctetStep {
    pub payload: Vec<u8>,
}

impl Serialize for GenericOctetStep {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        stream.write_u8(self.payload.len() as u8)?;
        stream.write(self.payload.as_slice())
    }
}

impl Deserialize for GenericOctetStep {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len = stream.read_u8()? as usize;
        let mut payload = vec![0u8; len];
        stream.read(&mut payload)?;
        Ok(Self { payload })
    }
}
