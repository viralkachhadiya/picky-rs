use crate::{
    private::SubjectPublicKeyInfo,
    x509::{
        private::{Name, Validity, Version},
        Extensions,
    },
    AlgorithmIdentifier,
};
use picky_asn1::wrapper::{ApplicationTag0, ApplicationTag3, BitStringAsn1, IntegerAsn1};
use serde::{de, Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Certificate {
    pub tbs_certificate: TBSCertificate,
    pub signature_algorithm: AlgorithmIdentifier,
    pub signature_value: BitStringAsn1,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct TBSCertificate {
    pub version: ApplicationTag0<Version>,
    pub serial_number: IntegerAsn1,
    pub signature: AlgorithmIdentifier,
    pub issuer: Name,
    pub validity: Validity,
    pub subject: Name,
    pub subject_public_key_info: SubjectPublicKeyInfo,
    // issuer_unique_id
    // subject_unique_id
    pub extensions: ApplicationTag3<Extensions>,
}

// Implement Deserialize manually to return an easy to understand error on V1 certificates
// (aka ApplicationTag0 not present).
impl<'de> de::Deserialize<'de> for TBSCertificate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = TBSCertificate;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct TBSCertificate")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                Ok(TBSCertificate {
                    version: seq
                        .next_element()
                        .map_err(|_| {
                            de::Error::invalid_value(
                                de::Unexpected::Other(
                                    "[TBSCertificate] V1 certificates unsupported. Only V3 certificates \
                                     are supported",
                                ),
                                &"a supported certificate",
                            )
                        })?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?,
                    serial_number: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?,
                    signature: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?,
                    issuer: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(3, &self))?,
                    validity: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(4, &self))?,
                    subject: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(5, &self))?,
                    subject_public_key_info: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(6, &self))?,
                    extensions: seq.next_element()?.ok_or_else(|| de::Error::invalid_length(7, &self))?,
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pem::parse_pem,
        x509::{
            extension::{KeyIdentifier, KeyUsage},
            name::DirectoryName,
            Cert, Extension,
        },
    };
    use num_bigint_dig::BigInt;
    use picky_asn1::{bit_string::BitString, date::UTCTime};

    #[test]
    fn x509_v3_certificate() {
        let encoded = base64::decode(
            "MIIEGjCCAgKgAwIBAgIEN8NXxDANBgkqhkiG9w0BAQsFADAiMSAwHgYDVQQ\
             DDBdjb250b3NvLmxvY2FsIEF1dGhvcml0eTAeFw0xOTEwMTcxNzQxMjhaFw0yMjEwM\
             TYxNzQxMjhaMB0xGzAZBgNVBAMMEnRlc3QuY29udG9zby5sb2NhbDCCASIwDQYJKoZ\
             IhvcNAQEBBQADggEPADCCAQoCggEBAMptALdk7xKj9JmFSycxlaTV47oLv5Aabir17\
             f1WseAcZ492Mx0wqcJMmT8rVAusyfqvrhodHu4GELGBySo4KChLEuoEOGTNw/wEMtM\
             6j1E9K7kig1iiuH9nf9oow7OUdix4+w7TWQWpwl1NekKdTtvLLtEGSjmG187CUqR6f\
             NHYag+iVMV5Umc5VQadvAgva8qxOsPpDkN/E2df5gST7H5g3igaZtxUa3x7VreN3qJ\
             P0+hYQiyM7KsgmdFAkKpHC6/k36H7SXtpzh0NbH5OJHifYsAP34WL+a6lAd0VM7UiI\
             RMcLWA8HfmKL3p4bC+LFv5I0dvUUy1BTz1wHpRvVz8CAwEAAaNdMFswCQYDVR0TBAI\
             wADAOBgNVHQ8BAf8EBAMCAaAwHQYDVR0OBBYEFCMimIgHf5c00sI9jZzeWoMLsR60M\
             B8GA1UdIwQYMBaAFBbHC24DEnsUFLz/zmqB5cMCHo9OMA0GCSqGSIb3DQEBCwUAA4I\
             CAQA1ehZTTBbes2DgGXwQugoV9PdOGMFEVT4dzrrluo/4exSfqLrNuY2NXVuNBKW4n\
             DA5aD71Q/KUZ8Y8cV9qa8OBJQvQ0dd0qeHmeEYdDsj5YD4ECycKx9U1ZX5fi6tpSIX\
             6DsietpCnrw4aTgbEOvMeQcuYCTP30Vpt+mYEKBlR/E2Vcl2zUD+67gqppSaC1RceL\
             /8Cy6ZXlPqwmS2zqK9UhYVRKlEww8xSh/9CR9MmIDc4pHtCpMawcn6Dmo+A+LcKi5v\
             /NIwvSJTei+h1gvRhvEOPcf4VZJMHXquNrxkMsKpuu7g/AYH7wl2MBaNaxyNlXY5e5\
             OjxslrbRCfDab11YaJEONcBnapl/+Ajr70uVFN09tDXyk0EHYf75NiRztgVKclna26\
             zP5qRb0JSYNQJW2kIIBX6DhU7kt6RcauF2hJ+jLWOF2vsAS8PdEr7vnR1EGOrrcQ3V\
             UgMscNsDqf50YMi2Inu1Kt2t+QSvYs61ON39aVpqR67nskdUWzFCVgWQVezM1ZagoO\
             yNp7WjRYl8hJ0YVZ7TRtP8nJOkZ6s046YHVWxMuGdqZfd/AUFb9xzzXjGRuuZ1JmSf\
             +VBOFEe2MaPMyMQBeIs3Othz6Fcy6Am5F6c3It31WYJwiCa/NdbMIvGy1xvAN5kzR/\
             Y6hkoQljoSr1rVuszJ9dtvuTccA==",
        )
        .expect("invalid base64");

        // Issuer

        let issuer: Name = DirectoryName::new_common_name("contoso.local Authority").into();
        check_serde!(issuer: Name in encoded[34..70]);

        // Validity

        let validity = Validity {
            not_before: UTCTime::new(2019, 10, 17, 17, 41, 28).unwrap().into(),
            not_after: UTCTime::new(2022, 10, 16, 17, 41, 28).unwrap().into(),
        };
        check_serde!(validity: Validity in encoded[70..102]);

        // Subject

        let subject: Name = DirectoryName::new_common_name("test.contoso.local").into();
        check_serde!(subject: Name in encoded[102..133]);

        // SubjectPublicKeyInfo

        let subject_public_key_info = SubjectPublicKeyInfo::new_rsa_key(
            IntegerAsn1::from(encoded[165..422].to_vec()),
            BigInt::from(65537).to_signed_bytes_be().into(),
        );
        check_serde!(subject_public_key_info: SubjectPublicKeyInfo in encoded[133..427]);

        // Extensions

        let mut key_usage = KeyUsage::new(7);
        key_usage.set_digital_signature(true);
        key_usage.set_key_encipherment(true);

        let extensions = Extensions(vec![
            Extension::new_basic_constraints(None, None).into_non_critical(),
            Extension::new_key_usage(key_usage),
            Extension::new_subject_key_identifier(&encoded[469..489]),
            Extension::new_authority_key_identifier(KeyIdentifier::from(encoded[502..522].to_vec()), None, None),
        ]);
        check_serde!(extensions: Extensions in encoded[429..522]);

        // SignatureAlgorithm

        let signature_algorithm = AlgorithmIdentifier::new_sha256_with_rsa_encryption();
        check_serde!(signature_algorithm: AlgorithmIdentifier in encoded[522..537]);

        // TBSCertificate

        let tbs_certificate = TBSCertificate {
            version: ApplicationTag0(Version::V3).into(),
            serial_number: BigInt::from(935548868).to_signed_bytes_be().into(),
            signature: signature_algorithm.clone(),
            issuer,
            validity,
            subject,
            subject_public_key_info,
            extensions: extensions.into(),
        };
        check_serde!(tbs_certificate: TBSCertificate in encoded[4..522]);

        // Full certificate

        let certificate = Certificate {
            tbs_certificate,
            signature_algorithm,
            signature_value: BitString::with_bytes(&encoded[542..1054]).into(),
        };
        check_serde!(certificate: Certificate in encoded);
    }

    #[test]
    fn key_id() {
        let intermediate_cert_pem = parse_pem(crate::test_files::INTERMEDIATE_CA).unwrap();
        let cert = Cert::from_der(intermediate_cert_pem.data()).unwrap();
        pretty_assertions::assert_eq!(
            hex::encode(&cert.subject_key_identifier().unwrap()),
            "1f74d63f29c17474453b05122c3da8bd435902a6"
        );
        pretty_assertions::assert_eq!(
            hex::encode(&cert.authority_key_identifier().unwrap().key_identifier().unwrap()),
            "b45ae4a5b3ded252f6b9d5a6950feb3ebcc7fdff"
        );
    }
}