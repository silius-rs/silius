use discv5::Enr;
use silius_p2p::discovery::enr_ext::EnrExt;
use std::str::FromStr;

#[test]
fn enr_decoding() {
    let enr_base64 = "enr:-KS4QOtEMvaUU7jVXuSONPateohOK2YfHsvLbA9S_PIaCw5HHct5Xl16mGg_JYndJEyyRwhvwwCTaFyOlyRLxscJybcFgmlkgnY0gmlwhCOy3eCPbWVtcG9vbF9zdWJuZXRziAAAAAAAAAAAiXNlY3AyNTZrMaECIuI8j36QBhWxb2DwNCdbDF3vpeKQ_CbaUpYJ9ltqfvyDdGNwghDxg3VkcIIQ8Q";
    let enr = <Enr>::from_str(enr_base64).unwrap();
    let binding = enr.multiaddr();
    let addr = binding.first().unwrap();
    assert_eq!(format!("{addr:?}"), "\"/ip4/35.178.221.224/tcp/4337\"");
}
