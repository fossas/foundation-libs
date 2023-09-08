use std::{fmt::Display, str::FromStr};

use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString};
use thiserror::Error;
use typed_builder::TypedBuilder;

/// Records all errors reported by this library.
#[derive(Error, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Errors encountered while parsing a [`Locator`].
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Errors encountered when parsing a [`Locator`] from a string.
#[derive(Error, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum ParseError {
    /// The input did not match the required syntax.
    #[error("input did not match required syntax: {input}")]
    Syntax {
        /// The input originally provided to the parser.
        input: String,
    },

    /// The "named" field was missing from the input.
    #[error("field '{field}' missing from input: {input}")]
    Field {
        /// The input originally provided to the parser.
        input: String,

        /// The field that was missing.
        field: String,
    },

    /// An unsupported value for the "fetcher" field was provided.
    /// Often this means that it is simply missing from this package.
    #[error("invalid fetcher '{fetcher}' in input '{input}'")]
    Fetcher {
        /// The input originally provided to the parser.
        input: String,

        /// The fetcher that was attempted to parse.
        fetcher: String,

        /// The error returned by the parser.
        #[source]
        error: strum::ParseError,
    },

    /// An unsupported value for the "project" field was provided.
    #[error("invalid project '{project}' in input '{input}'")]
    Project {
        /// The input originally provided to the parser.
        input: String,

        /// The project that was attempted to parse.
        project: String,

        /// The error returned by the parser.
        #[source]
        error: ProjectParseError,
    },
}

/// Core, and most services that interact with Core,
/// refer to open source packages via the `Locator` type.
///
/// This type is nearly universally rendered to a string
/// before being serialized to the database or sent over the network.
///
/// This type represents a _validly-constructed_ `Locator`, but does not
/// validate whether a `Locator` is actually valid. This means that a
/// given `Locator` is guaranteed to be correctly formatted data,
/// but that the actual repository or revision to which the `Locator`
/// refers is _not_ guaranteed to exist or be accessible.
/// Currently the canonical method for validating whether a given `Locator` is
/// accessible is to run it through the Core fetcher system.
///
/// For more information on the background of `Locator` and fetchers generally,
/// FOSSA employees may refer to
/// [Fetchers and Locators](https://go/fetchers-doc).
#[derive(Clone, Eq, PartialEq, Hash, Debug, TypedBuilder, Getters, CopyGetters)]
pub struct Locator {
    /// Determines which fetcher is used to download this project.
    #[getset(get_copy = "pub")]
    fetcher: Fetcher,

    /// Specifies the organization ID to which this project is namespaced.
    #[builder(default, setter(strip_option))]
    #[getset(get_copy = "pub")]
    org_id: Option<usize>,

    /// Specifies the unique identifier for the project by fetcher.
    ///
    /// For example, the `git` fetcher fetching a github project
    /// uses a value in the form of `{user_name}/{project_name}`.
    #[builder(setter(transform = |project: impl ToString| project.to_string()))]
    #[getset(get = "pub")]
    project: String,

    /// Specifies the version for the project by fetcher.
    ///
    /// For example, the `git` fetcher fetching a github project
    /// uses a value in the form of `{git_sha}` or `{git_tag}`,
    /// and the fetcher disambiguates.
    #[builder(default, setter(transform = |revision: impl ToString| Some(revision.to_string())))]
    #[getset(get = "pub")]
    revision: Option<String>,
}

impl Locator {
    /// Parse a `Locator`.
    ///
    /// The input string must be in one of the following forms:
    /// - `{fetcher}+{project}`
    /// - `{fetcher}+{project}$`
    /// - `{fetcher}+{project}${revision}`
    ///
    /// Projects may also be namespaced to a specific organization;
    /// in such cases the organization ID is at the start of the `{project}` field
    /// separated by a slash. The ID can be any non-negative integer.
    /// This yields the following formats:
    /// - `{fetcher}+{org_id}/{project}`
    /// - `{fetcher}+{org_id}/{project}$`
    /// - `{fetcher}+{org_id}/{project}${revision}`
    ///
    /// This parse function is based on the function used in FOSSA Core for maximal compatibility.
    pub fn parse(locator: &str) -> Result<Self, Error> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r"^(?:(?P<fetcher>[a-z-]+)\+|)(?P<project>[^$]+)(?:\$|)(?P<revision>.+|)$"
            )
            .expect("Locator parsing expression must compile");
        }

        let mut captures = RE.captures_iter(locator);
        let capture = captures.next().ok_or_else(|| ParseError::Syntax {
            input: locator.to_string(),
        })?;

        let fetcher =
            capture
                .name("fetcher")
                .map(|m| m.as_str())
                .ok_or_else(|| ParseError::Field {
                    input: locator.to_owned(),
                    field: "fetcher".to_string(),
                })?;

        let fetcher = Fetcher::try_from(fetcher).map_err(|error| ParseError::Fetcher {
            input: locator.to_owned(),
            fetcher: fetcher.to_string(),
            error,
        })?;

        let project = capture
            .name("project")
            .map(|m| m.as_str().to_owned())
            .ok_or_else(|| ParseError::Field {
                input: locator.to_owned(),
                field: "project".to_string(),
            })?;

        let revision = capture.name("revision").map(|m| m.as_str()).and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });

        match parse_org_project(&project) {
            Ok((org_id @ Some(_), project)) => Ok(Locator {
                fetcher,
                org_id,
                project: String::from(project),
                revision,
            }),
            Ok((org_id @ None, _)) => Ok(Locator {
                fetcher,
                org_id,
                project,
                revision,
            }),
            Err(error) => Err(Error::Parse(ParseError::Project {
                input: locator.to_owned(),
                project,
                error,
            })),
        }
    }

    /// Converts the locator into a [`PackageLocator`] by discarding the `revision` component.
    /// Equivalent to the `From` implementation, but offered as a method for convenience.
    pub fn into_package(self) -> PackageLocator {
        self.into()
    }
}

impl Display for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fetcher = &self.fetcher;
        write!(f, "{fetcher}+")?;

        let project = &self.project;
        if let Some(org_id) = &self.org_id {
            write!(f, "{org_id}/")?;
        }
        write!(f, "{project}")?;

        if let Some(revision) = &self.revision {
            write!(f, "${revision}")?;
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for Locator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Locator::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Locator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

/// A [`Locator`] specialized to not include the `revision` component.
///
/// Any [`Locator`] may be converted to a `PackageLocator` by simply discarding the `revision` component.
/// To create a [`Locator`] from a `PackageLocator`, the value for `revision` must be provided; see [`Locator`] for details.
#[derive(Clone, Eq, PartialEq, Hash, Debug, TypedBuilder)]
pub struct PackageLocator {
    /// Determines which fetcher is used to download this dependency
    /// from the internet.
    fetcher: Fetcher,

    /// Specifies the organization ID to which this project is namespaced.
    org_id: Option<usize>,

    /// Specifies the unique identifier for the project by fetcher.
    ///
    /// For example, the `git` fetcher fetching a github project
    /// uses a value in the form of `{user_name}/{project_name}`.
    #[builder(setter(transform = |project: impl ToString| project.to_string()))]
    project: String,
}

impl PackageLocator {
    /// Parse a `PackageLocator`.
    ///
    /// The input string must be in one of the following forms:
    /// - `{fetcher}+{project}`
    /// - `{fetcher}+{project}$`
    /// - `{fetcher}+{project}${revision}`
    ///
    /// Projects may also be namespaced to a specific organization;
    /// in such cases the organization ID is at the start of the `{project}` field
    /// separated by a slash. The ID can be any non-negative integer.
    /// This yields the following formats:
    /// - `{fetcher}+{org_id}/{project}`
    /// - `{fetcher}+{org_id}/{project}$`
    /// - `{fetcher}+{org_id}/{project}${revision}`
    ///
    /// This parse function is based on the function used in FOSSA Core for maximal compatibility.
    ///
    /// This implementation ignores the `revision` segment if it exists. If this is not preferred, use [`Locator`] instead.
    pub fn parse(locator: &str) -> Result<Self, Error> {
        let full = Locator::parse(locator)?;
        Ok(Self {
            fetcher: full.fetcher,
            org_id: full.org_id,
            project: full.project,
        })
    }

    /// Promote a `PackageLocator` to a [`Locator`] by providing the value to use for the `revision` component.
    pub fn promote(self, revision: Option<String>) -> Locator {
        Locator {
            fetcher: self.fetcher,
            org_id: self.org_id,
            project: self.project,
            revision,
        }
    }
}

impl Display for PackageLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let converted = Locator::from(self);
        write!(f, "{converted}")
    }
}

impl<'de> Deserialize<'de> for PackageLocator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        PackageLocator::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl Serialize for PackageLocator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

/// [`Locator`] is closely tied with the concept of Core's "fetchers",
/// which are asynchronous jobs tasked with downloading the code
/// referred to by a [`Locator`] so that Core or some other service
/// may analyze it.
///
/// `Fetcher` enumerates the identifiers of possible fetchers in Core.
/// The intention is not to maintain the list of _all possible fetchers_;
/// instead the intention is to provide a list of fetchers
/// with which services in this repository interact.
///
/// For this reason this enum is marked non-exhaustive and always will be:
/// it'll always be possible for fetchers to exist in Core or elsewhere
/// before they're enumerated here (by choice or by accident).
///
/// For more information on the background of `Locator` and fetchers generally,
/// refer to [Fetchers and Locators](https://go/fetchers-doc).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display, EnumString, EnumIter, AsRefStr)]
#[non_exhaustive]
pub enum Fetcher {
    /// The `git` fetcher handles interaction with git vcs hosts.
    #[strum(serialize = "git")]
    Git,

    /// The `custom` fetcher describes first party projects in FOSSA.
    ///
    /// These projects aren't really "fetched", they're just expressed
    /// this way in order to cooperate with the `Locator` shape.
    #[strum(serialize = "custom")]
    Custom,
}

impl<'de> Deserialize<'de> for Fetcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Fetcher::from_str(&raw).map_err(serde::de::Error::custom)
    }
}

impl Serialize for Fetcher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl From<Locator> for PackageLocator {
    fn from(full: Locator) -> Self {
        Self {
            fetcher: full.fetcher,
            org_id: full.org_id,
            project: full.project,
        }
    }
}

impl From<PackageLocator> for Locator {
    fn from(package: PackageLocator) -> Self {
        Self {
            fetcher: package.fetcher,
            org_id: package.org_id,
            project: package.project,
            revision: None,
        }
    }
}

impl From<&PackageLocator> for Locator {
    fn from(package: &PackageLocator) -> Self {
        package.clone().into()
    }
}

/// Errors encountered when parsing the project field
/// when parsing a [`Locator`] from a string.
#[derive(Error, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum ProjectParseError {
    /// An unsupported value for the "project" field was provided.
    #[error("project did not match required syntax: {project}")]
    Project {
        /// The project input.
        project: String,
    },

    /// The "named" field was missing from the input.
    #[error("field '{field}' missing from input: {project}")]
    Field {
        /// The input originally provided to the parser.
        project: String,

        /// The field that was missing.
        field: String,
    },
}

/// Optionally parse an org ID and trimmed project out of a project string.
fn parse_org_project(project: &str) -> Result<(Option<usize>, &str), ProjectParseError> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(?:(?P<org_id>\d+)/)?(?P<project>.+)")
            .expect("Project parsing expression must compile");
    }

    let mut captures = RE.captures_iter(project);
    let capture = captures.next().ok_or_else(|| ProjectParseError::Project {
        project: project.to_string(),
    })?;

    let trimmed_project =
        capture
            .name("project")
            .map(|m| m.as_str())
            .ok_or_else(|| ProjectParseError::Field {
                project: project.to_string(),
                field: String::from("project"),
            })?;

    // If we fail to parse the org_id as a valid number, don't fail the overall parse;
    // just don't namespace to org ID and return the input unmodified.
    match capture.name("org_id").map(|m| m.as_str()).map(str::parse) {
        // An org ID was provided and validly parsed, use it.
        Some(Ok(org_id)) => Ok((Some(org_id), trimmed_project)),

        // Otherwise, if we either didn't get an org ID section,
        // or it wasn't a valid org ID,
        // just use the project as-is.
        _ => Ok((None, project)),
    }
}

#[cfg(test)]
mod tests {
    use itertools::izip;

    use super::*;

    #[test]
    fn parses_org_project() {
        let orgs = [0usize, 1, 9809572];
        let names = ["name", "name/foo"];

        for (org, name) in izip!(orgs, names) {
            let test = format!("{org}/{name}");
            let Ok((Some(org_id), project)) = parse_org_project(&test) else {
                panic!("must parse '{test}'")
            };
            assert_eq!(org_id, org, "'org_id' must match in '{test}'");
            assert_eq!(project, name, "'project' must match in '{test}");
        }
    }

    #[test]
    fn parses_org_project_no_org() {
        let names = ["/name/foo", "/name", "abcd/1234/name", "1abc2/name"];
        for test in names {
            let Ok((org_id, project)) = parse_org_project(test) else {
                panic!("must parse '{test}'")
            };
            assert_eq!(org_id, None, "'org_id' must be None in '{test}'");
            assert_eq!(project, test, "'project' must match in '{test}");
        }
    }
}
