use crate::{Error, Additional, Extractor, http::Request};
use std::collections::HashMap;
use std::sync::Arc;

/// File contained in a multipart call
#[derive(Clone, Debug)]
pub struct File {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub content: Vec<u8>
}

/// Multipart extractor
pub struct Multipart {
    raw_files: HashMap<String, File>
}

impl Multipart {
    /// Retrieves a file from the multipart request by its name in the form
    pub fn file_by_name<A: AsRef<str>>(&self, name: A) -> Option<&File> {
        self.raw_files.get(name.as_ref())
    }

    /// Retrieves all files contained in the multipart as a vector
    pub fn files(&self) -> Vec<&File> {
        self.raw_files.iter().map(|(_n, v)| v).collect()
    }
}

impl<T: Sync> Extractor<T> for Multipart {
    fn extract(req: &Request, _additional: Arc<Additional<T>>) -> Result<Self, Error> {
        if let Some(content_type) = req.headers.get("Content-Type") {
            if let Some((multipart_tag, boundary_pair)) = content_type.trim().split_once(";") {
                if multipart_tag == "multipart/form-data" {
                    if let Some((tag, boundary)) = boundary_pair.trim().split_once("=") {
                        if tag == "boundary" {
                            // We create a pair of iterators, subsequent
                            let mut main_iter = req.content.iter().zip(req.content.iter().skip(1)).enumerate();
                            let mut parts: Vec<&[u8]> = Vec::new();

                            let mut previous = 0;
                            loop {
                                if let Some((idx, (one, two))) = main_iter.next() {
                                    if one == two && two == &b'-' {
                                        // Let's see if this is a boundary
                                        if idx + 2 + boundary.len() < req.content.len() {
                                            // We extract it
                                            if req.content.get(idx+2..idx+2+boundary.len()) == Some(boundary.as_bytes()) {
                                                // We advance the main_iter by the length of the boundary, and the remaining hyphen
                                                main_iter.nth(1 + boundary.len());

                                                // We add it to the parts vector
                                                parts.push(req.content.get(previous..idx).ok_or_else(|| Error::ExtractionBR(format!("internal error 1")))?);

                                                previous = idx + 2 + boundary.len();
                                            }
                                        }
                                    }
                                    // Else, we do nothing
                                } else {
                                    break;
                                }
                            }

                            // The first token needs to contain nothing
                            if parts.len() > 0 && parts[0].is_empty() {
                               parts.drain(0..1); 
                            } else {
                                return Err(Error::ExtractionBR(format!("the content of the multipart request does not start properly")));
                            }

                            // If this multiform is properly formatted, then it needs to finish in `--\r\n`
                            if !(previous < req.content.len() && req.content.get(previous..req.content.len()).unwrap() == vec![b'-', b'-', b'\r', b'\n']) {
                                return Err(Error::ExtractionBR(format!("the content of the multipart request does not finish properly")));
                            }

                            // Now, for each token, we will remove the first and last 2 characters, which must be `\r\n`
                            for (idx, part) in parts.iter_mut().enumerate() {
                                if part.len() < 4 || part.get(0..2) != Some(&[b'\r', b'\n']) || part.get(part.len()-2..part.len()) != Some(&[b'\r', b'\n']) {
                                    return Err(Error::ExtractionBR(format!("part {} of the multipart is not properly finished or started", idx)));
                                }
                                *part = part.get(2..part.len()-2).ok_or_else(|| Error::ExtractionBR(format!("internal error 2")))?;
                            }

                            // File holder
                            let mut raw_files = HashMap::new();

                            // Now, we will turn each part into a file
                            for part in parts {
                                let secondary_iter = part.iter().zip(part.iter().skip(2)).enumerate();
                                let mut split_index = None;
                                for (idx, (a, b)) in secondary_iter {
                                    if a==b && b==&b'\n' && idx > 0 && part[idx-1] == b'\r' && part[idx+1] == b'\r' {
                                        split_index = Some(idx);
                                        break;
                                    }
                                }

                                let split_index = split_index.ok_or(Error::ExtractionBR(format!("no end of inner-header was found for multipart")))?;

                                // We split one character before, because of the `\r`. This operation is safe, due to the secondary_iter search
                                let (inner_header, inner_content) = part.split_at(split_index - 1);
                                // The header needs to be a string
                                let inner_header = String::from_utf8(inner_header.to_vec()).map_err(|e| Error::ExtractionBR(format!("incorrect inner header format, {}", e)))?;
                                // We have to remove the `\r\n\r\n` that is at the beginning of the remaining bytes
                                let (_, inner_content) = inner_content.split_at(4);

                                let mut file = File {
                                    name: "".into(),
                                    filename: None,
                                    content_type: None,
                                    content: inner_content.to_vec()
                                };
                                let mut name_set = false;

                                for line in inner_header.split("\r\n") {
                                    if let Some((tag, details)) = line.split_once(": ") {
                                        match tag {
                                            "Content-Disposition" => {
                                                let mut token_iter = details.split("; ");
                                                let mut pairs = HashMap::new();
                                                // The first token needs to be "form-data"
                                                if let Some(form_data_candidate) = token_iter.next() {
                                                    if form_data_candidate != "form-data" {
                                                        return Err(Error::ExtractionBR(format!("each document in multiform must be form-data content type")))
                                                    }

                                                    for remaining_token in token_iter {
                                                        if let Some((key, value)) = remaining_token.split_once("=") {
                                                            pairs.insert(key, value);
                                                        }
                                                    }

                                                    file.name = pairs.get("name").ok_or_else(|| Error::ExtractionBR(format!("a name for the multipart was not found")))?.to_string();

                                                    file.filename = pairs.get("filename").map(|v| {
                                                        if v.len() > 2 {
                                                            v[1..v.len()-1].to_string()
                                                        } else {
                                                            v.to_string()
                                                        }
                                                    });
                                                    name_set = true;
                                                } else {
                                                    return Err(Error::ExtractionBR(format!("Content-Disposition header seems to be empty")))
                                                }
                                            },
                                            "Content-Type" => {
                                                file.content_type = Some(details.into());
                                            },
                                            _ => ()
                                        }
                                    } else {
                                        return Err(Error::ExtractionBR(format!("malformed header")))
                                    }
                                }

                                if !name_set {
                                    return Err(Error::ExtractionBR(format!("a component of the multipart was found to have no name")));
                                }
                
                                raw_files.insert(file.name.clone(), file);
                            }
                            
                            Ok(Multipart {
                                raw_files
                            })
                        } else {
                            Err(Error::ExtractionBR(format!("boundary tag was not found")))
                        }
                    } else {
                        Err(Error::ExtractionBR(format!("the boundary should be specified as `boundary=???`")))
                    }
                } else {
                    Err(Error::ExtractionBR(format!("multipart content-type must be multipart/form-data (received `{}`)", multipart_tag)))
                }
            } else {
                Err(Error::ExtractionBR(format!("multipart content-type requires the multipart/form-data tag, and a boundary")))
            }
        } else {
            Err(Error::ExtractionBR(format!("multipart request requires the content-type header")))
        }
    }
}