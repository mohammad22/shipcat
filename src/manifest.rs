use serde_yaml;
use regex::Regex;

use std::io::prelude::*;
use std::fs::File;
use std::path::{PathBuf, Path};
use std::collections::BTreeMap;
use std::fmt;

use super::Result;
use super::vault::Vault;

// k8s related structs

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceRequest {
    /// CPU request string
    cpu: String,
    /// Memory request string
    memory: String,
    // TODO: ephemeral-storage + extended-resources
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceLimit {
    /// CPU limit string
    cpu: String,
    /// Memory limit string
    memory: String,
    // TODO: ephemeral-storage + extended-resources
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Resources {
    /// Resource requests for k8s
    pub requests: Option<ResourceRequest>,
    /// Resource limits for k8s
    pub limits: Option<ResourceLimit>,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Replicas {
    /// Minimum replicas for k8s deployment
    pub min: u32,
    /// Maximum replicas for k8s deployment
    pub max: u32,
}


#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ConfigMappedFile {
    /// Name of file to template (from service repo paths)
    pub name: String,
    /// Name of file inside container
    pub dest: String,
}
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ConfigMap {
    /// Optional k8s specific name for the mount (autogenerated if left out)
    pub name: Option<String>,
    /// Container-local directory path where configs are available
    pub mount: String,
    /// Files from the config map to mount at this mountpath
    pub files: Vec<ConfigMappedFile>
}

// misc structs

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dashboard {
    /// Metric strings to track
    pub rows: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Prometheus {
    /// Whether to poll
    pub enabled: bool,
    /// Path to poll
    pub path: String,
    // TODO: Maybe include names of metrics?
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dependency {
    /// Name of service relied upon (used to goto dependent manifest)
    pub name: String,
    // TODO: api name - should be in the dependent manifest
    /// API version relied upon (v1 default)
    pub api: Option<String>,
    // other metadata?
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Image {
    /// Name of service relied upon
    pub name: Option<String>,
    /// Repository to fetch the image from (can be empty string)
    pub repository: Option<String>,
    /// Tag to fetch the image from (defaults to latest)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}
impl fmt::Display for Image {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prefix = self.repository.clone().map(|s| {
            if s != "" { format!("{}/", s) } else { s }
        }).unwrap_or_else(|| "".into());
        let suffix = self.tag.clone().unwrap_or_else(|| "latest".to_string());
        // NB: assume image.name is always set at this point
        write!(f, "{}{}:{}", prefix, self.name.clone().unwrap(), suffix)
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InitContainer {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VolumeSecretItem {
    #[serde(default)]
    pub key: String,
    pub path: String,
    #[serde(default)]
    pub mode: u32,
}
impl Default for VolumeSecretItem {
    fn default() -> VolumeSecretItem {
        VolumeSecretItem {
            key: "value".to_string(),
            path: "/".to_string(),
            mode: 420,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct VolumeSecretDetail {
    pub name: String,
    pub items: Vec<VolumeSecretItem>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct VolumeSecret {
    pub secret: Option<VolumeSecretDetail>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProjectedVolumeSecret {
    pub sources: Vec<VolumeSecret>,
    // pub default_mode: u32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Volume {
    pub name: String,
    /// A projection combines multiple volume items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected: Option<ProjectedVolumeSecret>,
    /// The secret is fetched  from kube secrets and mounted as a volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<VolumeSecretDetail>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultOpts {
    /// If Vault name differs from service name
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct HealthCheck {
    // Port where the health check is located (typically first exposed port)
    //pub port: u32,
    // NB: maybe do ports later, currently use first exposed port

    /// Where the health check is located (typically /health)
    pub uri: String,
    /// How long to wait after boot in seconds (typically 30s)
    pub wait: u32
}



/// Main manifest, serializable from shipcat.yml
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    /// Name of the service
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional image name (if different from service name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    /// Optional image command (if not using the default docker command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    // Kubernetes specific flags

    /// Resource limits and requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources>,
    /// Replication limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<Replicas>,
    /// Environment variables to inject
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Config files to inline in a configMap
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configs: Option<ConfigMap>,
    /// Init container intructions
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub init_containers: Vec<InitContainer>,
    /// Ports to expose
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<u32>,
    /// Vault options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultOpts>,
    /// Health check parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthCheck>,
    /// Service dependencies
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
    /// Regions service is deployed to
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
    /// Volumes
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,

    // TODO: boot time -> minReadySeconds

// TODO: service dependencies!

    /// Prometheus metric options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prometheus: Option<Prometheus>,
//prometheus:
//  enabled: true
//  path: /metrics
    /// Dashboards to generate
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dashboards: BTreeMap<String, Dashboard>,
//dashboards:
//  auth-python:
//    rows:
//      - users-connected
//      - conversation-length

// TODO: logging alerts
//logging:
//  alerts:
//    error-rate-5xx:
//      type: median
//      threshold: 2
//      status-code: 500
//preStopHookPath: /die
// newrelic options? we generate the newrelic.ini from a vault secret + manifest.name

    // Internal path of this manifest
    #[serde(skip_serializing, skip_deserializing)]
    _location: String,

//    // Parsed port map of this manifest
//    #[serde(skip_serializing, skip_deserializing)]
//    pub _portmaps: Vec<PortMap>
}


impl Manifest {
    pub fn new(name: &str, location: &PathBuf) -> Manifest {
        Manifest {
            name: Some(name.into()),
            _location: location.to_string_lossy().into(),
            ..Default::default()
        }
    }
    /// Read a manifest file in an arbitrary path
    fn read_from(pwd: &PathBuf) -> Result<Manifest> {
        let mpath = pwd.join("shipcat.yml");
        trace!("Using manifest in {}", mpath.display());
        if !mpath.exists() {
            bail!("Manifest file {} does not exist", mpath.display())
        }
        let mut f = File::open(&mpath)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mut res: Manifest = serde_yaml::from_str(&data)?;
        // store the location internally (not serialized to disk)
        res._location = mpath.to_string_lossy().into();
        Ok(res)
    }

    /// Add implicit defaults to self
    fn implicits(&mut self) -> Result<()> {
        let name = self.name.clone().unwrap();

        // image name defaults to the service name
        if self.image.is_none() {
            self.image = Some(Image {
                name: Some(name.clone()),
                repository: None,
                tag: None,
            });
        }

        // config map implicit name
        if let Some(ref mut cfg) = self.configs {
            if cfg.name.is_none() {
                cfg.name = Some(format!("{}-config", name.clone()));
            }
        }

        for d in &mut self.dependencies {
            if d.api.is_none() {
                d.api = Some("v1".to_string());
            }
        }

        Ok(())
    }

    /// Merge defaults from partial override file
    fn merge(&mut self, pth: &PathBuf) -> Result<()> {
        trace!("Merging {}", pth.display());
        if !pth.exists() {
            bail!("Defaults file {} does not exist", pth.display())
        }
        let name = self.name.clone().unwrap();
        let mut f = File::open(&pth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;
        let mf: Manifest = serde_yaml::from_str(&data)?;

        for (k,v) in mf.env {
            self.env.entry(k).or_insert(v);
        }

        if let Some(img) = mf.image {
            // allow overriding default repository and tags
            let mut curr = self.image.clone().unwrap();
            if curr.repository.is_none() {
                trace!("overriding image.repository with {:?}", img.repository);
                curr.repository = img.repository;
            }
            if curr.tag.is_none() {
                trace!("overriding image.tag with {:?}", img.tag);
                curr.tag = img.tag;
            }
            self.image = Some(curr);
        }

        if self.resources.is_none() && mf.resources.is_some() {
            self.resources = mf.resources.clone();
        }
        if let Some(ref mut res) = self.resources {
            if res.limits.is_none() {
                res.limits = mf.resources.clone().unwrap().limits;
            }
            if res.requests.is_none() {
                res.requests = mf.resources.clone().unwrap().requests;
            }
            // for now: if limits or requests are specified, you have to fill in both CPU and memory
        }
        if !self.init_containers.is_empty() && !mf.init_containers.is_empty() {
            self.init_containers = mf.init_containers.clone();
        }
        if self.replicas.is_none() && mf.replicas.is_some() {
            self.replicas = mf.replicas;
        }
        if self.ports.is_empty() {
            warn!("{} exposes no ports", name.clone());
        }
        if self.health.is_none() && mf.health.is_some() {
            self.health = mf.health
        }
        if self.volumes.is_empty() && !mf.volumes.is_empty() {
            self.volumes = mf.volumes;
        }

        // only using target ports now, disabling this now
        //for s in &self.ports {
        //    self._portmaps.push(parse_ports(s)?);
        //}
        Ok(())
    }

    // Populate placeholder fields with secrets from vault
    fn secrets(&mut self, client: &mut Vault, env: &str, loc: &str) -> Result<()> {
        // some services use keys from other services
        let svc = if let Some(ref vopts) = self.vault {
            vopts.name.clone()
        } else {
            self.name.clone().unwrap()
        };

        // iterate over key value evars and replace placeholders
        for (k, v) in &mut self.env {
            let kube_prefix = "IN_KUBE_SECRETS";

            if v == "IN_VAULT" {
                let vkey = format!("{}-{}/{}/{}", env, loc, svc, k);
                let secret = client.read(&vkey)?;
                *v = secret;
            } else if v.starts_with(kube_prefix) {
                let res = if v == kube_prefix {
                    // no extra info -> assume same kube secret name as evar name
                    k.to_string()
                } else {
                    // key after :, split and return second half
                    assert!(v.contains(':'));
                    let parts : Vec<_> = v.split(':').collect();
                    if parts[1].is_empty() {
                        bail!("{} does not have a valid key path", v.clone());
                    }
                    parts[1].to_string()
                };
                *v = format!("kube-secret-{}", res.to_lowercase().replace("_", "-"));
            }
        }
        Ok(())
    }

    // Return a completed (read, filled in, and populate secrets) manifest
    pub fn completed(env: &str, loc: &str, service: &str, vault: Option<&mut Vault>) -> Result<Manifest> {
        let pth = Path::new(".").join("services").join(service);
        if !pth.exists() {
            bail!("Service folder {} does not exist", pth.display())
        }
        let mut mf = Manifest::read_from(&pth)?;
        mf.implicits()?;
        if let Some(client) = vault {
            debug!("Injecting secrets from vault {}-{}", env, loc);
            mf.secrets(client, env, loc)?;
        }

        // merge service specific env overrides if they exists
        let envlocals = Path::new(".")
            .join("services")
            .join(service)
            .join(format!("{}-{}.yml", env, loc));
        if envlocals.is_file() {
            debug!("Merging environment locals from {}", envlocals.display());
            mf.merge(&envlocals)?;
        }
        // merge global environment defaults if they exist
        let envglobals = Path::new(".")
            .join("environments")
            .join(format!("{}-{}.yml", env, loc));
        if envglobals.is_file() {
            debug!("Merging environment globals from {}", envglobals.display());
            mf.merge(&envglobals)?;
        }
        Ok(mf)
    }

    /// Update the manifest file in the current folder
    pub fn write(&self) -> Result<()> {
        let encoded = serde_yaml::to_string(self)?;
        trace!("Writing manifest in {}", self._location);
        let mut f = File::create(&self._location)?;
        write!(f, "{}\n", encoded)?;
        debug!("Wrote manifest in {}: \n{}", self._location, encoded);
        Ok(())
    }

    /// Print manifest to stdout
    pub fn print(&self) -> Result<()> {
        let encoded = serde_yaml::to_string(self)?;
        print!("{}\n", encoded);
        Ok(())
    }

    /// Verify assumptions about manifest
    ///
    /// Assumes the manifest has been populated with `implicits`
    pub fn verify(&self) -> Result<()> {
        if self.name.is_none() || self.name.clone().unwrap() == "" {
            bail!("Name cannot be empty")
        }
        let name = self.name.clone().unwrap();

        // 1. Verify resources
        // (We can unwrap all the values as we assume implicit called!)
        let req = self.resources.clone().unwrap().requests.unwrap().clone();
        let lim = self.resources.clone().unwrap().limits.unwrap().clone();
        let req_memory = parse_memory(&req.memory)?;
        let lim_memory = parse_memory(&lim.memory)?;
        let req_cpu = parse_cpu(&req.cpu)?;
        let lim_cpu = parse_cpu(&lim.cpu)?;

        // 1.1 limits >= requests
        if req_cpu > lim_cpu {
            bail!("Requested more CPU than what was limited");
        }
        if req_memory > lim_memory {
            bail!("Requested more memory than what was limited");
        }
        // 1.2 sanity numbers
        if req_cpu > 10.0 {
            bail!("Requested more than 10 cores");
        }
        if req_memory > 10.0*1024.0*1024.0*1024.0 {
            bail!("Requested more than 10 GB of memory");
        }
        if lim_cpu > 20.0 {
            bail!("CPU limit set to more than 20 cores");
        }
        if lim_memory > 20.0*1024.0*1024.0*1024.0 {
            bail!("Memory limit set to more than 20 GB of memory");
        }

        // 2. Ports restrictions? currently parse only

        // 3. configs
        // 3.1) mount paths can't be empty string
        if let Some(ref cfgmap) = self.configs {
            if cfgmap.mount == "" || cfgmap.mount == "~" {
                bail!("Empty mountpath for {} mount ", cfgmap.name.clone().unwrap())
            }
            if !cfgmap.mount.ends_with('/') {
                bail!("Mount path '{}' for {} must end with a slash", cfgmap.mount, cfgmap.name.clone().unwrap());
            }
            for f in &cfgmap.files {
                if !f.name.ends_with(".j2") {
                    bail!("Only supporting templated config files atm")
                }
                // TODO: verify file exists? done later anyway
            }
        } else {
            warn!("No configs key in manifest");
            warn!("Did you use the old volumes key?");
        }

        // 4. volumes
        // TODO:

        // 5. dependencies
        for d in &self.dependencies {
            // 5.a) d.name must exist in services/
            let dpth = Path::new(".").join("services").join(d.name.clone());
            if !dpth.is_dir() {
                bail!("Service {} does not exist in services/", d.name);
            }
            // 5.b) d.api must parse as an integer
            assert!(d.api.is_some(), "api version set by implicits");
            if let Some(ref apiv) = d.api {
                let vstr = apiv.chars().skip_while(|ch| *ch == 'v').collect::<String>();
                let ver : usize = vstr.parse()?;
                trace!("Parsed api version of dependency {} as {}", d.name.clone(), ver);
            }
        }

        // 6. regions must have a defaults file in ./environments
        for r in &self.regions {
            let regionfile = Path::new(".")
                .join("environments")
                .join(format!("{}.yml", r));

            if ! regionfile.is_file() {
                bail!("Unsupported region {} without region file {}",
                    r, regionfile.display());
            }
        }
        if self.regions.is_empty() {
            bail!("No regions specified for {}", name);
        }

        // 7. init containers - only verify syntax
        if !self.init_containers.is_empty() {
            for init_container in &self.init_containers {
                let re = Regex::new(r"(?:[a-z]+/)?([a-z]+)(?::[0-9]+)?").unwrap();
                if !re.is_match(&init_container.image) {
                    bail!("The init container {} does not seem to match a valid image registry", init_container.name);
                }
                if init_container.command.is_empty() {
                    bail!("A command must be specified for the init container {}", init_container.name);
                }
            }
        }

        // 8. dependencies
        // verify that auto-injected keys are not overriding
        // TODO: maybe something for another implicits like thing
        // TODO: verify dependencies exist in service repo

        // X. TODO: other keys

        Ok(())
    }
}

// Parse normal k8s memory resource value into floats
fn parse_memory(s: &str) -> Result<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let mut res : f64 = digits.parse()?;
    trace!("Parsed {} ({})", digits, unit);
    if unit == "Ki" {
        res *= 1024.0;
    } else if unit == "Mi" {
        res *= 1024.0*1024.0;
    } else if unit == "Gi" {
        res *= 1024.0*1024.0*1024.0;
    } else if unit == "k" {
        res *= 1000.0;
    } else if unit == "M" {
        res *= 1000.0*1000.0;
    } else if unit == "G" {
        res *= 1000.0*1000.0*1000.0;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} bytes", res);
    Ok(res)
}

// Parse normal k8s cpu resource values into floats
// We don't allow power of two variants here
fn parse_cpu(s: &str) -> Result<f64> {
    let digits = s.chars().take_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let unit = s.chars().skip_while(|ch| ch.is_digit(10) || *ch == '.').collect::<String>();
    let mut res : f64 = digits.parse()?;

    trace!("Parsed {} ({})", digits, unit);
    if unit == "m" {
        res /= 1000.0;
    } else if unit == "k" {
        res *= 1000.0;
    } else if unit != "" {
        bail!("Unknown unit {}", unit);
    }
    trace!("Returned {} cores", res);
    Ok(res)
}


pub fn validate(env: &str, location: &str, service: &str) -> Result<()> {
    let mf = Manifest::completed(env, location, service, None)?;
    mf.verify()?;
    mf.print()
}
