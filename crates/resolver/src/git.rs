use tokio::process::Command;

use crate::ResolverError;

#[derive(Debug, Clone)]
pub struct GitDependency<'a> {
    pub host: &'a str,
    pub owner: &'a str,
    pub repo: &'a str,
    pub reference: Ref<'a>,
}

impl<'a> GitDependency<'a> {
    pub fn from_url(url: &'a str, reference: Ref<'a>) -> Option<Self> {
        let url = url.strip_suffix(".git").unwrap_or(url);
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() < 3 {
            return None;
        }
        let host = parts[0];
        let owner = parts[1];
        let repo = parts[2];
        Some(Self {
            host,
            owner,
            repo,
            reference,
        })
    }

    pub fn tag(url: &'a str, tag: &'a str) -> Option<Self> {
        Self::from_url(url, Ref::Tag(tag))
    }

    pub fn branch(url: &'a str, branch: &'a str) -> Option<Self> {
        Self::from_url(url, Ref::Branch(branch))
    }

    pub fn rev(url: &'a str, rev: &'a str) -> Option<Self> {
        Self::from_url(url, Ref::Rev(rev))
    }

    pub fn git_url(&self) -> String {
        format!("https://{}/{}/{}.git", self.host, self.owner, self.repo)
    }

    pub async fn resolve_commit(&self) -> Result<String, ResolverError> {
        let refspec = match self.reference {
            Ref::Rev(sha) => return Ok(sha.to_string()),
            Ref::Tag(t) => format!("refs/tags/{t}"),
            Ref::Branch(b) => format!("refs/heads/{b}"),
        };

        let output = Command::new("git")
            .arg("ls-remote")
            .arg(self.git_url())
            .arg(&refspec)
            .output()
            .await
            .map_err(|e| ResolverError::GitLsRemote(self.repo.to_string(), e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(ResolverError::GitLsRemoteFailed(
                self.repo.to_string(),
                stderr,
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_ls_remote(&stdout)
            .ok_or_else(|| ResolverError::GitRefNotFound(self.repo.to_string(), refspec))
    }

    pub fn commit_archive_url(&self, sha: &str) -> String {
        match self.host {
            "gitlab.com" => format!(
                "https://gitlab.com/{}/{}/-/archive/{}/{}-{}.tar.gz",
                self.owner, self.repo, sha, self.repo, sha
            ),
            "github.com" => format!(
                "https://github.com/{}/{}/archive/{}.tar.gz",
                self.owner, self.repo, sha
            ),
            host => format!(
                "https://{}/{}/{}/archive/{}.tar.gz",
                host, self.owner, self.repo, sha
            ),
        }
    }

    pub fn archive_url(&self) -> Option<String> {
        match self.host {
            "github.com" => Some(match self.reference {
                Ref::Tag(t) => {
                    format!(
                        "https://github.com/{}/{}/archive/refs/tags/{}.tar.gz",
                        self.owner, self.repo, t
                    )
                }
                Ref::Branch(b) => {
                    format!(
                        "https://github.com/{}/{}/archive/refs/heads/{}.tar.gz",
                        self.owner, self.repo, b
                    )
                }
                Ref::Rev(s) => format!(
                    "https://github.com/{}/{}/archive/{}.tar.gz",
                    self.owner, self.repo, s
                ),
            }),
            "gitlab.com" => match self.reference {
                Ref::Tag(t) => Some(format!(
                    "https://gitlab.com/{}/{}/-/archive/{}/{}-{}.tar.gz",
                    self.owner, self.repo, t, self.repo, t
                )),
                Ref::Branch(b) => Some(format!(
                    "https://gitlab.com/{}/{}/-/archive/{}/{}-{}.tar.gz",
                    self.owner, self.repo, b, self.repo, b
                )),
                Ref::Rev(_) => None,
            },
            _ => Some(match self.reference {
                Ref::Tag(t) | Ref::Branch(t) | Ref::Rev(t) => {
                    format!(
                        "https://{}/{}/{}/archive/{}.tar.gz",
                        self.host, self.owner, self.repo, t
                    )
                }
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Ref<'a> {
    Tag(&'a str),
    Branch(&'a str),
    Rev(&'a str),
}

impl Ref<'_> {
    pub fn name(&self) -> &str {
        match self {
            Ref::Tag(_) => "tag",
            Ref::Branch(_) => "branch",
            Ref::Rev(_) => "rev",
        }
    }
}

fn parse_ls_remote(output: &str) -> Option<String> {
    let mut fallback = None;
    for line in output.lines() {
        let Some((sha, refname)) = line.split_once('\t') else {
            continue;
        };
        if refname.ends_with("^{}") {
            return Some(sha.to_string());
        }
        fallback.get_or_insert_with(|| sha.to_string());
    }
    fallback
}
