//! core/ifcx_collab.rs — Phase 80 (`v0.8.0`): Collaborative Multi-Layer IFCX Governance,
//! Cryptographically Chained Checkpoints, Deterministic Three-Way Merge, & Coupled District Risk Field.
//!
//! Provides:
//! - Tamper-evident SHA-256 checkpoint chain (`IfcxCheckpointRecord`) linking base BIM, FEM/ARI v2,
//!   Geotechnical SSI, SHM/ATC-20, Evacuation, and Monte Carlo IFCX layers across disciplines
//! - Formal engineering sign-off workflow (`Draft` -> `InReview` -> `Approved` / `Superseded`)
//! - Portable `.ifcx-bundle.json` team handover package with end-to-end integrity verification
//! - Deterministic Three-Way IFCX Node & Attribute Merge (`Base`, `Local`, `Incoming`) with
//!   per-conflict resolution (`KeepLocal`, `KeepIncoming`, `KeepBothLayered`)
//! - Non-destructive governance overlay (`eigengraph::governance::checkpoint_seal`)
//! - Spatial Inverse-Distance-Weighted (IDW) District Risk Field evaluator uniting ARI v2, ATC-20,
//!   FEMA P-58 PML-90, Seed–Idriss liquefaction, and A* evacuation isolation over `IfcSite` TIN terrain.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::bim::BimProject;
use crate::core::evacuation_routing::EvacuationLifelineReport;
use crate::core::geotech::SubsurfaceSsiAnalysisReport;
use crate::core::gis::map_widget::SlippyMapState;
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::world_index::WorldSpatialIndex;
use crate::core::gpu_fragility_mc::MonteCarloFragilityReport;
use crate::core::ifcx::{ns, IfcxDocument, IfcxNode};
use crate::core::interop::Sha256Hasher;
use crate::core::shm_twin::{Atc20Placard, ShmDigitalTwinReport};
use crate::models::StructuralMember;

/// Non-destructive IFCX namespace for signed engineering governance & checkpoint provenance.
pub const NS_EG_GOVERNANCE_SEAL: &str = ns::EG_GOVERNANCE_SEAL;

/// Engineering discipline owning a collaborative `.ifcx` layer or checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CollabDiscipline {
    #[default]
    Structural,
    Geotechnical,
    Architecture,
    EmergencyOps,
    PeerReview,
}

impl CollabDiscipline {
    pub fn label(self) -> &'static str {
        match self {
            Self::Structural => "Structural Engineering",
            Self::Geotechnical => "Geotechnical & SSI",
            Self::Architecture => "Architecture & Core",
            Self::EmergencyOps => "Emergency & Lifelines",
            Self::PeerReview => "Independent Peer Review",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Structural => "Structural",
            Self::Geotechnical => "Geotech",
            Self::Architecture => "Arch",
            Self::EmergencyOps => "Emergency",
            Self::PeerReview => "PeerReview",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Structural,
            Self::Geotechnical,
            Self::Architecture,
            Self::EmergencyOps,
            Self::PeerReview,
        ]
    }

    pub fn next(self) -> Self {
        match self {
            Self::Structural => Self::Geotechnical,
            Self::Geotechnical => Self::Architecture,
            Self::Architecture => Self::EmergencyOps,
            Self::EmergencyOps => Self::PeerReview,
            Self::PeerReview => Self::Structural,
        }
    }

    pub fn badge_color_rgb(self) -> [u8; 3] {
        match self {
            Self::Structural => [56, 189, 248],
            Self::Geotechnical => [245, 158, 11],
            Self::Architecture => [167, 139, 250],
            Self::EmergencyOps => [244, 63, 94],
            Self::PeerReview => [16, 185, 129],
        }
    }
}

/// Formal review & engineering sign-off lifecycle state for an IFCX checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CheckpointApprovalStatus {
    #[default]
    Draft,
    InReview,
    Approved,
    Superseded,
}

impl CheckpointApprovalStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::InReview => "IN REVIEW",
            Self::Approved => "APPROVED",
            Self::Superseded => "SUPERSEDED",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::InReview => "REVIEW",
            Self::Approved => "APPROVED",
            Self::Superseded => "SUPERSEDED",
        }
    }

    pub fn badge_color_rgb(self) -> [u8; 3] {
        match self {
            Self::Draft => [148, 163, 184],
            Self::InReview => [245, 158, 11],
            Self::Approved => [16, 185, 129],
            Self::Superseded => [100, 116, 139],
        }
    }
}

/// Per-attribute resolution strategy during a three-way `.ifcx` merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConflictResolution {
    #[default]
    KeepLocal,
    KeepIncoming,
    KeepBothLayered,
}

impl ConflictResolution {
    pub fn label(self) -> &'static str {
        match self {
            Self::KeepLocal => "Keep Local",
            Self::KeepIncoming => "Keep Incoming",
            Self::KeepBothLayered => "Keep Both (Layered)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::KeepLocal => "Local",
            Self::KeepIncoming => "Incoming",
            Self::KeepBothLayered => "Layered",
        }
    }
}

/// Switchable physical & financial metric projected on the 3D Pure WebGPU District Risk Field Heatmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DistrictRiskMetric {
    #[default]
    CompositeRisk,
    McPml90Loss,
    StructuralAri,
    EvacuationIsolation,
}

impl DistrictRiskMetric {
    pub fn label(self) -> &'static str {
        match self {
            Self::CompositeRisk => "Composite Multi-Hazard Risk",
            Self::McPml90Loss => "FEMA P-58 PML-90 Loss Density",
            Self::StructuralAri => "Structural ARI v2 Vulnerability",
            Self::EvacuationIsolation => "Post-Quake Isolation & ATC-20 Cordon",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::CompositeRisk => "Composite",
            Self::McPml90Loss => "PML-90",
            Self::StructuralAri => "ARI Risk",
            Self::EvacuationIsolation => "Isolation",
        }
    }

    pub fn unit_label(self) -> &'static str {
        match self {
            Self::CompositeRisk => "0-100% Index",
            Self::McPml90Loss => "kUSD PML-90",
            Self::StructuralAri => "ARI Deficit %",
            Self::EvacuationIsolation => "Isolation %",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::CompositeRisk,
            Self::McPml90Loss,
            Self::StructuralAri,
            Self::EvacuationIsolation,
        ]
    }
}

/// Engineering KPI snapshot frozen inside every signed IFCX checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct IfcxCheckpointKpi {
    pub min_ari: f64,
    pub max_dcr_pct: f64,
    pub atc20_placard: String,
    pub mc_pml90_usd: f64,
    pub evacuation_conn_pct: f64,
    pub node_count: usize,
}

/// Single cryptographically chained engineering checkpoint record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcxCheckpointRecord {
    pub checkpoint_id: String,
    pub seq: u64,
    pub parent_id: Option<String>,
    pub label: String,
    pub author: String,
    pub discipline: CollabDiscipline,
    pub approval_status: CheckpointApprovalStatus,
    pub approved_by: Option<String>,
    pub kpi: IfcxCheckpointKpi,
    pub payload_sha256: String,
    pub chain_sha256: String,
    pub snapshot: IfcxDocument,
}

fn write_canonical_json(out: &mut String, val: &Value) {
    match val {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else if let Some(f) = n.as_f64() {
                let v = if f == 0.0 { 0.0 } else { f };
                if !v.is_finite() {
                    out.push_str("null");
                } else if v.fract() == 0.0 && v.abs() < 1e15 {
                    out.push_str(&format!("{v:.1}"));
                } else {
                    out.push_str(&format!("{v:.9}"));
                }
            }
        }
        Value::String(s) => {
            if let Ok(esc) = serde_json::to_string(s) {
                out.push_str(&esc);
            }
        }
        Value::Array(arr) => {
            out.push('[');
            for (idx, item) in arr.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (idx, k) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                if let Ok(esc_k) = serde_json::to_string(k) {
                    out.push_str(&esc_k);
                }
                out.push(':');
                if let Some(v) = map.get(*k) {
                    write_canonical_json(out, v);
                }
            }
            out.push('}');
        }
    }
}

fn canonical_sha256_of_serializable<T: Serialize>(val: &T) -> Result<String, String> {
    let json_val = serde_json::to_value(val).map_err(|e| e.to_string())?;
    let mut canonical = String::with_capacity(1024);
    write_canonical_json(&mut canonical, &json_val);
    Ok(Sha256Hasher::digest_hex(canonical.as_bytes()))
}

impl IfcxCheckpointRecord {
    fn compute_payload_hash(snapshot: &IfcxDocument) -> String {
        canonical_sha256_of_serializable(snapshot).unwrap_or_default()
    }

    fn compute_chain_hash(
        parent_chain: Option<&str>,
        seq: u64,
        checkpoint_id: &str,
        author: &str,
        discipline: CollabDiscipline,
        approval_status: CheckpointApprovalStatus,
        approved_by: Option<&str>,
        payload_sha256: &str,
    ) -> String {
        let canonical = format!(
            "parent={}|seq={seq}|id={checkpoint_id}|author={author}|disc={:?}|status={:?}|approver={}|payload={payload_sha256}",
            parent_chain.unwrap_or("GENESIS"),
            discipline,
            approval_status,
            approved_by.unwrap_or("NONE"),
        );
        Sha256Hasher::digest_hex(canonical.as_bytes())
    }

    /// Recomputes `chain_sha256` against a supplied parent chain digest without altering snapshot bytes.
    pub fn recompute_chain_hash(&mut self, parent_chain: Option<&str>) {
        self.chain_sha256 = Self::compute_chain_hash(
            parent_chain,
            self.seq,
            &self.checkpoint_id,
            &self.author,
            self.discipline,
            self.approval_status,
            self.approved_by.as_deref(),
            &self.payload_sha256,
        );
    }

    /// Verifies both canonical payload digest and cryptographic parent-chain link.
    pub fn verify_integrity(&self, expected_parent_chain: Option<&str>) -> bool {
        let actual_payload = Self::compute_payload_hash(&self.snapshot);
        if actual_payload != self.payload_sha256 {
            return false;
        }
        let expected_chain = Self::compute_chain_hash(
            expected_parent_chain,
            self.seq,
            &self.checkpoint_id,
            &self.author,
            self.discipline,
            self.approval_status,
            self.approved_by.as_deref(),
            &self.payload_sha256,
        );
        expected_chain == self.chain_sha256
    }

    /// Returns short 12-character hex prefix of `chain_sha256` for UI badges and manifests.
    pub fn short_sha256(&self) -> &str {
        if self.chain_sha256.len() >= 12 {
            &self.chain_sha256[..12]
        } else {
            &self.chain_sha256
        }
    }
}

/// Attribute-level conflict detected during a three-way `.ifcx` merge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcxMergeConflict {
    pub path: String,
    pub attribute_key: String,
    pub base_val: Option<Value>,
    pub local_val: Option<Value>,
    pub incoming_val: Option<Value>,
    pub resolution: ConflictResolution,
}

/// Complete outcome of a deterministic three-way `.ifcx` merge across Base, Local, and Incoming revisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcxThreeWayMergeResult {
    pub merged_doc: IfcxDocument,
    pub base_label: String,
    pub local_label: String,
    pub incoming_label: String,
    pub auto_merged_nodes: usize,
    pub conflicts: Vec<IfcxMergeConflict>,
    pub merged_sha256: String,
    pub summary: String,
}

/// Signed `.ifcx-bundle.json` team handover package encapsulating full checkpoint provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcxCollabBundle {
    pub format_version: String,
    pub project_name: String,
    pub checkpoints: Vec<IfcxCheckpointRecord>,
    pub bundle_sha256: String,
}

impl IfcxCollabBundle {
    fn compute_bundle_hash(
        format_version: &str,
        project_name: &str,
        checkpoints: &[IfcxCheckpointRecord],
    ) -> Result<String, String> {
        #[derive(Serialize)]
        struct CanonicalBundle<'a> {
            format_version: &'a str,
            project_name: &'a str,
            checkpoints: &'a [IfcxCheckpointRecord],
        }
        canonical_sha256_of_serializable(&CanonicalBundle {
            format_version,
            project_name,
            checkpoints,
        })
        .map_err(|e| format!("Bundle serialization failed: {e}"))
    }

    /// Serializes a signed checkpoint chain bundle into verified pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, String> {
        let mut copy = self.clone();
        copy.bundle_sha256 = Self::compute_bundle_hash(
            &copy.format_version,
            &copy.project_name,
            &copy.checkpoints,
        )?;
        serde_json::to_string_pretty(&copy).map_err(|e| format!("Bundle JSON write failed: {e}"))
    }

    /// Parses and verifies both outer bundle SHA-256 seal and inner checkpoint chain hashes.
    pub fn from_verified_json(raw: &str) -> Result<Self, String> {
        let bundle: Self = serde_json::from_str(raw)
            .map_err(|e| format!("Invalid .ifcx-bundle.json syntax: {e}"))?;
        let expected = Self::compute_bundle_hash(
            &bundle.format_version,
            &bundle.project_name,
            &bundle.checkpoints,
        )?;
        if expected != bundle.bundle_sha256 {
            return Err("Bundle SHA-256 seal mismatch: handover package was altered.".to_string());
        }
        let (ok, failed) = verify_checkpoint_chain(&bundle.checkpoints);
        if !ok {
            return Err(format!(
                "Checkpoint cryptographic chain verification failed at {failed} link(s)."
            ));
        }
        Ok(bundle)
    }
}

/// Ranked parcel or asset hotspot in the spatial District Risk Field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistrictRiskHotspot {
    pub name: String,
    pub rel_path: String,
    pub lat: f64,
    pub lon: f64,
    pub east_m: f32,
    pub north_m: f32,
    pub risk_score: f32,
    pub metric_value: f64,
    pub min_ari: f64,
    pub max_dcr: f64,
    pub pml90_usd: f64,
    pub atc20_tag: String,
}

/// Single terrain-draped cell in the IDW spatial District Risk Field mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistrictRiskGridCell {
    pub ix: usize,
    pub iy: usize,
    pub cx_m: f32,
    pub cy_m: f32,
    pub half_step_m: f32,
    pub elev_m: f32,
    pub risk_01: f32,
}

/// Complete spatial IDW District Risk Field evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DistrictRiskFieldReport {
    pub metric: DistrictRiskMetric,
    pub grid_n: usize,
    pub domain_half_m: f32,
    pub cells: Vec<DistrictRiskGridCell>,
    pub hotspots: Vec<DistrictRiskHotspot>,
    pub mean_risk_01: f32,
    pub peak_risk_01: f32,
    pub high_risk_area_pct: f32,
    pub low_count: usize,
    pub moderate_count: usize,
    pub high_count: usize,
    pub critical_count: usize,
    pub evaluated_buildings: usize,
    pub content_hash: u64,
    pub summary: String,
}

/// Persisted workspace configuration for Collaborative IFCX Governance & 3D Risk Heatmap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxCollabConfig {
    pub default_author: String,
    pub active_discipline: CollabDiscipline,
    pub risk_heatmap_visible: bool,
    pub risk_metric: DistrictRiskMetric,
    pub risk_grid_resolution: usize,
    pub risk_opacity: f32,
}

impl Default for IfcxCollabConfig {
    fn default() -> Self {
        Self {
            default_author: "Lead Structural Engineer".to_string(),
            active_discipline: CollabDiscipline::Structural,
            risk_heatmap_visible: false,
            risk_metric: DistrictRiskMetric::CompositeRisk,
            risk_grid_resolution: 22,
            risk_opacity: 0.68,
        }
    }
}

impl IfcxCollabConfig {
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, val: u64| {
            *acc ^= val;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, u64::from(self.risk_heatmap_visible) + 1);
        mix(&mut h, self.risk_metric as u64 + 17);
        mix(&mut h, self.risk_grid_resolution as u64);
        mix(&mut h, (self.risk_opacity * 1000.0).round() as u64);
        mix(&mut h, self.active_discipline as u64 + 31);
        h
    }
}

/// Interactive workspace controller for signed `.ifcx` checkpoints, 3-way merges, and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcxCollabState {
    pub config: IfcxCollabConfig,
    pub checkpoints: Vec<IfcxCheckpointRecord>,
    pub active_checkpoint_idx: Option<usize>,
    pub last_merge: Option<IfcxThreeWayMergeResult>,
    pub peer_branch_label: String,
    pub peer_discipline: CollabDiscipline,
    pub checkpoint_label_draft: String,
    #[serde(default = "default_relay_url")]
    pub relay_url_draft: String,
    #[serde(default)]
    pub relay_connected: bool,
    #[serde(default)]
    pub relay_status: String,
    #[serde(default)]
    pub relay_synced_count: usize,
}

fn default_relay_url() -> String {
    "wss://relay.eigengraph.net/ifcx-room".to_string()
}

impl Default for IfcxCollabState {
    fn default() -> Self {
        Self {
            config: IfcxCollabConfig::default(),
            checkpoints: Vec::new(),
            active_checkpoint_idx: None,
            last_merge: None,
            peer_branch_label: "Geotech & Peer Branch".to_string(),
            peer_discipline: CollabDiscipline::PeerReview,
            checkpoint_label_draft: "Design Milestone".to_string(),
            relay_url_draft: default_relay_url(),
            relay_connected: false,
            relay_status: "Offline (Ready)".to_string(),
            relay_synced_count: 0,
        }
    }
}

/// Verifies integrity across every record in a checkpoint slice.
pub fn verify_checkpoint_chain(checkpoints: &[IfcxCheckpointRecord]) -> (bool, usize) {
    let mut prev_chain: Option<&str> = None;
    let mut invalid = 0usize;
    for cp in checkpoints {
        if !cp.verify_integrity(prev_chain) {
            invalid += 1;
        }
        prev_chain = Some(cp.chain_sha256.as_str());
    }
    (invalid == 0, invalid)
}

impl IfcxCollabState {
    /// Creates and chains a new tamper-evident SHA-256 IFCX checkpoint record.
    pub fn create_checkpoint(
        &mut self,
        label: &str,
        discipline: CollabDiscipline,
        snapshot: IfcxDocument,
        kpi: IfcxCheckpointKpi,
    ) -> &IfcxCheckpointRecord {
        let seq = (self.checkpoints.len() as u64) + 1;
        let checkpoint_id = format!("CP-{seq:03}");
        let clean_label = if label.trim().is_empty() {
            format!("{} Milestone #{seq}", discipline.short_label())
        } else {
            label.trim().to_string()
        };
        let parent_id = self.checkpoints.last().map(|c| c.checkpoint_id.clone());
        let parent_chain = self.checkpoints.last().map(|c| c.chain_sha256.clone());
        let payload_sha256 = IfcxCheckpointRecord::compute_payload_hash(&snapshot);
        let chain_sha256 = IfcxCheckpointRecord::compute_chain_hash(
            parent_chain.as_deref(),
            seq,
            &checkpoint_id,
            &self.config.default_author,
            discipline,
            CheckpointApprovalStatus::Draft,
            None,
            &payload_sha256,
        );
        let rec = IfcxCheckpointRecord {
            checkpoint_id,
            seq,
            parent_id,
            label: clean_label,
            author: self.config.default_author.clone(),
            discipline,
            approval_status: CheckpointApprovalStatus::Draft,
            approved_by: None,
            kpi,
            payload_sha256,
            chain_sha256,
            snapshot,
        };
        self.checkpoints.push(rec);
        let idx = self.checkpoints.len() - 1;
        self.active_checkpoint_idx = Some(idx);
        &self.checkpoints[idx]
    }

    /// Recomputes `chain_sha256` links sequentially from genesis after status mutations.
    pub fn rechain_from_genesis(&mut self) {
        let mut prev: Option<String> = None;
        for cp in &mut self.checkpoints {
            cp.recompute_chain_hash(prev.as_deref());
            prev = Some(cp.chain_sha256.clone());
        }
    }

    /// Signs off the active checkpoint (`Approved`), superseding any older `Approved` checkpoint.
    pub fn approve_active_checkpoint(&mut self, approver: &str) -> Result<String, String> {
        if self.checkpoints.is_empty() {
            return Err("No IFCX checkpoint exists yet. Create one first with /checkpoint.".to_string());
        }
        let target_idx = self
            .active_checkpoint_idx
            .unwrap_or_else(|| self.checkpoints.len() - 1)
            .min(self.checkpoints.len() - 1);
        let sign_name = if approver.trim().is_empty() {
            "Principal Structural Reviewer".to_string()
        } else {
            approver.trim().to_string()
        };
        for (i, cp) in self.checkpoints.iter_mut().enumerate() {
            if i != target_idx && cp.approval_status == CheckpointApprovalStatus::Approved {
                cp.approval_status = CheckpointApprovalStatus::Superseded;
            }
        }
        {
            let target = &mut self.checkpoints[target_idx];
            target.approval_status = CheckpointApprovalStatus::Approved;
            target.approved_by = Some(sign_name.clone());
        }
        self.rechain_from_genesis();
        self.active_checkpoint_idx = Some(target_idx);
        let cp = &self.checkpoints[target_idx];
        Ok(format!(
            "Signed & approved {} '{}' ({}) by {} • Chain SHA-256: {}",
            cp.checkpoint_id,
            cp.label,
            cp.discipline.short_label(),
            sign_name,
            cp.short_sha256()
        ))
    }

    /// Returns the currently selected (or latest) checkpoint record.
    pub fn active_checkpoint(&self) -> Option<&IfcxCheckpointRecord> {
        if self.checkpoints.is_empty() {
            return None;
        }
        let idx = self
            .active_checkpoint_idx
            .unwrap_or_else(|| self.checkpoints.len() - 1)
            .min(self.checkpoints.len() - 1);
        self.checkpoints.get(idx)
    }

    /// Verifies cryptographic payload and chain integrity across all checkpoints.
    pub fn verify_chain(&self) -> (bool, usize) {
        verify_checkpoint_chain(&self.checkpoints)
    }

    /// Exports all checkpoints into a verified `.ifcx-bundle.json` team handover package.
    pub fn export_bundle_json(&self, project_name: &str) -> Result<String, String> {
        let bundle = IfcxCollabBundle {
            format_version: "EIGEN-IFCX-COLLAB-1.0".to_string(),
            project_name: project_name.to_string(),
            checkpoints: self.checkpoints.clone(),
            bundle_sha256: String::new(),
        };
        bundle.to_pretty_json()
    }

    /// Imports and cryptographically verifies a `.ifcx-bundle.json` team handover package.
    pub fn import_bundle_json(&mut self, json: &str) -> Result<usize, String> {
        let bundle = IfcxCollabBundle::from_verified_json(json)?;
        let count = bundle.checkpoints.len();
        self.checkpoints = bundle.checkpoints;
        self.active_checkpoint_idx = count.checked_sub(1);
        Ok(count)
    }

    /// Reconciles an incoming peer `.ifcx` delta received from the Collaborative Sync Relay
    /// via deterministic 3-Way Merge against `base_doc` and `local_doc`.
    pub fn ingest_relay_peer_doc(
        &mut self,
        base_doc: &IfcxDocument,
        local_doc: &IfcxDocument,
        peer_author: &str,
        peer_disc: CollabDiscipline,
        incoming_doc: &IfcxDocument,
    ) -> String {
        let merge = three_way_merge_ifcx(
            base_doc,
            local_doc,
            incoming_doc,
            "Base",
            "Local",
            &format!("{peer_author} ({})", peer_disc.short_label()),
        );
        let msg = merge.summary.clone();
        self.last_merge = Some(merge);
        self.relay_synced_count += 1;
        self.relay_status = format!("Synced #{} from {}", self.relay_synced_count, peer_author);
        msg
    }

    /// Builds a non-destructive IFCX governance provenance overlay node
    /// (`eigengraph::governance::checkpoint_seal`) when at least one checkpoint exists.
    pub fn build_governance_seal_overlay(&self, project: &BimProject) -> Option<IfcxDocument> {
        let cp = self.active_checkpoint()?;
        let (chain_valid, invalid_count) = self.verify_chain();
        let mut doc = IfcxDocument::new(&format!("{}_governance_seal", project.project_name));
        let mut attrs = BTreeMap::new();
        attrs.insert(
            ns::BSI_CLASS.to_string(),
            json!({ "code": "IfcApproval", "version": "IFC5-IFCX" }),
        );
        attrs.insert(
            ns::BSI_PROP.to_string(),
            json!({
                "name": format!("Governance {} ({})", cp.checkpoint_id, cp.approval_status.short_label()),
                "discipline": cp.discipline.label(),
                "author": cp.author,
            }),
        );
        attrs.insert(
            ns::EG_GOVERNANCE_SEAL.to_string(),
            json!({
                "checkpoint_id": cp.checkpoint_id,
                "seq": cp.seq,
                "parent_id": cp.parent_id,
                "label": cp.label,
                "discipline": cp.discipline.short_label(),
                "approval_status": cp.approval_status.label(),
                "author": cp.author,
                "approved_by": cp.approved_by,
                "payload_sha256": cp.payload_sha256,
                "chain_sha256": cp.chain_sha256,
                "chain_valid": chain_valid,
                "invalid_links": invalid_count,
                "total_checkpoints": self.checkpoints.len(),
                "kpi": cp.kpi,
            }),
        );
        doc.data.push(IfcxNode {
            path: "/earth/governance/checkpoint_seal".to_string(),
            children: BTreeMap::new(),
            inherits: BTreeMap::new(),
            attributes: attrs,
        });
        Some(doc)
    }
}

fn index_ifcx_nodes(doc: &IfcxDocument) -> BTreeMap<String, IfcxNode> {
    let mut map = BTreeMap::new();
    for node in &doc.data {
        map.insert(node.path.clone(), node.clone());
    }
    map
}

fn merge_btreemap_values<V: Clone + PartialEq>(
    base: &BTreeMap<String, V>,
    local: &BTreeMap<String, V>,
    incoming: &BTreeMap<String, V>,
) -> BTreeMap<String, V> {
    let mut keys: Vec<String> = base
        .keys()
        .chain(local.keys())
        .chain(incoming.keys())
        .cloned()
        .collect();
    keys.sort();
    keys.dedup();
    let mut out = BTreeMap::new();
    for k in keys {
        let b = base.get(&k);
        let l = local.get(&k);
        let r = incoming.get(&k);
        let chosen = if l == r {
            l.cloned()
        } else if l == b {
            r.cloned()
        } else if r == b {
            l.cloned()
        } else {
            l.cloned().or_else(|| r.cloned())
        };
        if let Some(v) = chosen {
            out.insert(k, v);
        }
    }
    out
}

/// Performs deterministic Three-Way Attribute & Node Merge across `base`, `local`, and `incoming`
/// `.ifcx` scene graphs, recording non-trivial concurrent edits as explicit `IfcxMergeConflict` items.
pub fn three_way_merge_ifcx(
    base: &IfcxDocument,
    local: &IfcxDocument,
    incoming: &IfcxDocument,
    base_lbl: &str,
    local_lbl: &str,
    incoming_lbl: &str,
) -> IfcxThreeWayMergeResult {
    let base_map = index_ifcx_nodes(base);
    let local_map = index_ifcx_nodes(local);
    let incoming_map = index_ifcx_nodes(incoming);

    let mut all_paths: Vec<String> = base_map
        .keys()
        .chain(local_map.keys())
        .chain(incoming_map.keys())
        .cloned()
        .collect();
    all_paths.sort();
    all_paths.dedup();

    let mut merged_doc = IfcxDocument::new(&format!("{}_merged", local.header.id));
    merged_doc.imports = local.imports.clone();
    merged_doc.schemas = local.schemas.clone();
    for (k, v) in &incoming.schemas {
        merged_doc
            .schemas
            .entry(k.clone())
            .or_insert_with(|| v.clone());
    }

    let mut auto_merged_nodes = 0usize;
    let mut conflicts = Vec::new();

    for path in all_paths {
        let b_opt = base_map.get(&path);
        let l_opt = local_map.get(&path);
        let r_opt = incoming_map.get(&path);

        // Node deletion / addition three-way rule
        let present_local = l_opt.is_some();
        let present_incoming = r_opt.is_some();
        let present_base = b_opt.is_some();
        if !present_local && !present_incoming {
            continue;
        }
        if present_base && !present_local && present_incoming {
            // Deleted locally, unchanged or modified remotely
            if r_opt == b_opt {
                auto_merged_nodes += 1;
                continue;
            }
        }
        if present_base && present_local && !present_incoming && l_opt == b_opt {
            auto_merged_nodes += 1;
            continue;
        }
        if l_opt.is_none() && b_opt.is_none() {
            if let Some(rn) = r_opt {
                merged_doc.data.push(rn.clone());
                auto_merged_nodes += 1;
                continue;
            }
        }
        if r_opt.is_none() && b_opt.is_none() {
            if let Some(ln) = l_opt {
                merged_doc.data.push(ln.clone());
                auto_merged_nodes += 1;
                continue;
            }
        }

        let empty_children = BTreeMap::new();
        let empty_inherits = BTreeMap::new();
        let empty_attrs = BTreeMap::new();
        let b_ch = b_opt.map(|n| &n.children).unwrap_or(&empty_children);
        let l_ch = l_opt.map(|n| &n.children).unwrap_or(&empty_children);
        let r_ch = r_opt.map(|n| &n.children).unwrap_or(&empty_children);
        let b_inh = b_opt.map(|n| &n.inherits).unwrap_or(&empty_inherits);
        let l_inh = l_opt.map(|n| &n.inherits).unwrap_or(&empty_inherits);
        let r_inh = r_opt.map(|n| &n.inherits).unwrap_or(&empty_inherits);
        let b_at = b_opt.map(|n| &n.attributes).unwrap_or(&empty_attrs);
        let l_at = l_opt.map(|n| &n.attributes).unwrap_or(&empty_attrs);
        let r_at = r_opt.map(|n| &n.attributes).unwrap_or(&empty_attrs);

        let merged_children = merge_btreemap_values(b_ch, l_ch, r_ch);
        let merged_inherits = merge_btreemap_values(b_inh, l_inh, r_inh);

        let mut attr_keys: Vec<String> = b_at
            .keys()
            .chain(l_at.keys())
            .chain(r_at.keys())
            .cloned()
            .collect();
        attr_keys.sort();
        attr_keys.dedup();

        let mut merged_attrs = BTreeMap::new();
        let mut node_had_conflict = false;
        for ak in attr_keys {
            let bv = b_at.get(&ak);
            let lv = l_at.get(&ak);
            let rv = r_at.get(&ak);
            if lv == rv {
                if let Some(v) = lv {
                    merged_attrs.insert(ak, v.clone());
                }
            } else if lv == bv {
                if let Some(v) = rv {
                    merged_attrs.insert(ak, v.clone());
                }
            } else if rv == bv {
                if let Some(v) = lv {
                    merged_attrs.insert(ak, v.clone());
                }
            } else {
                node_had_conflict = true;
                conflicts.push(IfcxMergeConflict {
                    path: path.clone(),
                    attribute_key: ak.clone(),
                    base_val: bv.cloned(),
                    local_val: lv.cloned(),
                    incoming_val: rv.cloned(),
                    resolution: ConflictResolution::KeepLocal,
                });
                if let Some(v) = lv.or(rv) {
                    merged_attrs.insert(ak, v.clone());
                }
            }
        }
        if !node_had_conflict {
            auto_merged_nodes += 1;
        }
        merged_doc.data.push(IfcxNode {
            path,
            children: merged_children,
            inherits: merged_inherits,
            attributes: merged_attrs,
        });
    }

    let bytes = serde_json::to_vec(&merged_doc).unwrap_or_default();
    let merged_sha256 = Sha256Hasher::digest_hex(&bytes);
    let summary = format!(
        "3-Way IFCX Merge ({local_lbl} + {incoming_lbl} on {base_lbl}): {} auto-merged nodes, {} conflicts • SHA {}...",
        auto_merged_nodes,
        conflicts.len(),
        &merged_sha256[..12.min(merged_sha256.len())]
    );
    IfcxThreeWayMergeResult {
        merged_doc,
        base_label: base_lbl.to_string(),
        local_label: local_lbl.to_string(),
        incoming_label: incoming_lbl.to_string(),
        auto_merged_nodes,
        conflicts,
        merged_sha256,
        summary,
    }
}

/// Applies an interactive per-conflict resolution choice (`KeepLocal`, `KeepIncoming`, or `KeepBothLayered`)
/// to an existing `IfcxThreeWayMergeResult` and updates its cryptographic SHA-256 digest.
pub fn apply_conflict_resolution(
    merge: &mut IfcxThreeWayMergeResult,
    conflict_idx: usize,
    resolution: ConflictResolution,
) {
    let Some(c) = merge.conflicts.get_mut(conflict_idx) else {
        return;
    };
    c.resolution = resolution;
    let resolved_val = match resolution {
        ConflictResolution::KeepLocal => c.local_val.clone(),
        ConflictResolution::KeepIncoming => c.incoming_val.clone(),
        ConflictResolution::KeepBothLayered => Some(json!({
            "merge_mode": "layered",
            "local": c.local_val.clone().unwrap_or(Value::Null),
            "incoming": c.incoming_val.clone().unwrap_or(Value::Null),
            "base": c.base_val.clone().unwrap_or(Value::Null),
        })),
    };
    if let Some(node) = merge.merged_doc.data.iter_mut().find(|n| n.path == c.path) {
        match resolved_val {
            Some(v) => {
                node.attributes.insert(c.attribute_key.clone(), v);
            }
            None => {
                node.attributes.remove(&c.attribute_key);
            }
        }
    }
    let bytes = serde_json::to_vec(&merge.merged_doc).unwrap_or_default();
    merge.merged_sha256 = Sha256Hasher::digest_hex(&bytes);
    merge.summary = format!(
        "3-Way IFCX Merge ({} + {} on {}): {} auto-merged nodes, {} conflicts • SHA {}...",
        merge.local_label,
        merge.incoming_label,
        merge.base_label,
        merge.auto_merged_nodes,
        merge.conflicts.len(),
        &merge.merged_sha256[..12.min(merge.merged_sha256.len())]
    );
}

/// Synthesizes a realistic multi-discipline peer `.ifcx` branch revision from a base document,
/// adding discipline-owned nodes (clean auto-merge) alongside a controlled property revision
/// so multi-discipline three-way merge workflows can be tested or demonstrated in one click.
pub fn synthesize_peer_branch_doc(
    base_doc: &IfcxDocument,
    peer_discipline: CollabDiscipline,
) -> IfcxDocument {
    let mut peer = base_doc.clone();
    peer.header.id = format!(
        "{}_peer_{}",
        base_doc.header.id,
        peer_discipline.short_label().to_ascii_lowercase()
    );
    peer.header.author = format!("Peer Team ({})", peer_discipline.label());

    // Mutate the first structural building/element metadata node deterministically
    if let Some(first_elem) = peer
        .data
        .iter_mut()
        .find(|n| n.attributes.contains_key(ns::EG_BIM_PROJECT) || n.attributes.contains_key(ns::BSI_PROP))
    {
        let mut prop = first_elem
            .attributes
            .get(ns::BSI_PROP)
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Some(obj) = prop.as_object_mut() {
            obj.insert(
                "peer_review_note".to_string(),
                json!(format!(
                    "{} concurrence check: ductile detailing verified",
                    peer_discipline.short_label()
                )),
            );
            obj.insert(
                "discipline_revision".to_string(),
                json!(format!("{}-R2", peer_discipline.short_label().to_uppercase())),
            );
        }
        first_elem.attributes.insert(ns::BSI_PROP.to_string(), prop);
    }

    // Add a clean discipline-specific layer node that auto-merges cleanly
    let mut peer_attrs = BTreeMap::new();
    peer_attrs.insert(
        ns::BSI_CLASS.to_string(),
        json!({ "code": "IfcSpatialZone", "version": "IFC5-IFCX" }),
    );
    peer_attrs.insert(
        ns::BSI_PROP.to_string(),
        json!({
            "name": format!("{} Peer Layer", peer_discipline.label()),
            "discipline": peer_discipline.short_label(),
            "status": "SubmittedForCoordination"
        }),
    );
    peer.data.push(IfcxNode {
        path: format!(
            "/earth/collab/peer_{}",
            peer_discipline.short_label().to_ascii_lowercase()
        ),
        children: BTreeMap::new(),
        inherits: BTreeMap::new(),
        attributes: peer_attrs,
    });
    peer
}

struct BuildingRiskSource {
    name: String,
    rel_path: String,
    lat: f64,
    lon: f64,
    east_m: f32,
    north_m: f32,
    min_ari: f64,
    max_dcr: f64,
    pml90_usd: f64,
    atc20_tag: String,
    risk_composite: f32,
    risk_pml90: f32,
    risk_ari: f32,
    risk_isolation: f32,
}

fn placard_severity_01(placard: Atc20Placard) -> f32 {
    match placard {
        Atc20Placard::InspectedGreen => 0.14,
        Atc20Placard::RestrictedYellow => 0.56,
        Atc20Placard::UnsafeRed => 0.94,
    }
}

/// Evaluates an Inverse-Distance-Weighted (IDW) spatial District Risk Field draped over `IfcSite` TIN
/// relief, coupling ARI v2, ATC-20 placards, FEMA P-58 Monte Carlo PML-90, Seed–Idriss liquefaction,
/// and A* evacuation isolation across the active BIM asset and surrounding `WorldSpatialIndex` buildings.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_district_risk_field(
    config: &IfcxCollabConfig,
    origin_lat_lon: (f64, f64),
    bim_project: &BimProject,
    members: &[StructuralMember],
    world_index: &WorldSpatialIndex,
    active_site_tin: Option<&TinSurfaceMesh>,
    ssi_opt: Option<&SubsurfaceSsiAnalysisReport>,
    shm_opt: Option<&ShmDigitalTwinReport>,
    mc_opt: Option<&MonteCarloFragilityReport>,
    evac_opt: Option<&EvacuationLifelineReport>,
) -> DistrictRiskFieldReport {
    let active_min_ari = members
        .iter()
        .map(|m| m.ari)
        .fold(1.0_f64, f64::min)
        .clamp(0.0, 1.0);
    let active_max_dcr = members
        .iter()
        .map(|m| m.utilization)
        .fold(0.35_f64, f64::max)
        .clamp(0.0, 2.5);
    let liq_boost = if ssi_opt.is_some_and(|s| s.is_liquefaction_critical) {
        0.16_f32
    } else if ssi_opt.is_some_and(|s| s.period_lengthening_pct > 15.0) {
        0.07_f32
    } else {
        0.0_f32
    };
    let evac_isolation_global = evac_opt
        .map(|e| {
            let disconn = ((100.0 - e.network_connectivity_pct) / 100.0).clamp(0.0, 1.0) as f32;
            let iso_ratio = if e.isolated_building_count > 0 { 0.35 } else { 0.0 };
            (disconn * 0.72 + iso_ratio).clamp(0.05, 0.98)
        })
        .unwrap_or(0.18);

    let active_placard = shm_opt
        .map(|s| s.atc20_placard)
        .unwrap_or(if active_min_ari < 0.65 || active_max_dcr > 1.05 {
            Atc20Placard::UnsafeRed
        } else if active_min_ari < 0.80 || active_max_dcr > 0.85 {
            Atc20Placard::RestrictedYellow
        } else {
            Atc20Placard::InspectedGreen
        });

    let active_pml90 = mc_opt
        .map(|mc| mc.p90_loss_usd)
        .unwrap_or_else(|| {
            let elem_est = (bim_project.columns.len() + bim_project.runs.len()).max(8) as f64;
            elem_est * 9_500.0 * (1.25 - active_min_ari).clamp(0.30, 1.50)
        });
    let active_pml90_01 = ((active_pml90 / 850_000.0).clamp(0.05, 1.0)) as f32;
    let active_ari_risk = ((1.0 - active_min_ari).clamp(0.0, 1.0) * 0.68
        + (active_max_dcr / 1.35).clamp(0.0, 1.0) * 0.32) as f32;
    let active_iso_risk =
        (evac_isolation_global * 0.60 + placard_severity_01(active_placard) * 0.40).clamp(0.05, 1.0);
    let active_comp = (active_ari_risk * 0.36
        + placard_severity_01(active_placard) * 0.22
        + active_pml90_01 * 0.18
        + active_iso_risk * 0.14
        + liq_boost)
        .clamp(0.04, 0.99);

    let mut sources = Vec::new();
    sources.push(BuildingRiskSource {
        name: if bim_project.project_name.trim().is_empty() {
            "Active BIM Structure".to_string()
        } else {
            bim_project.project_name.clone()
        },
        rel_path: "_active_project_".to_string(),
        lat: origin_lat_lon.0,
        lon: origin_lat_lon.1,
        east_m: 0.0,
        north_m: 0.0,
        min_ari: active_min_ari,
        max_dcr: active_max_dcr,
        pml90_usd: active_pml90,
        atc20_tag: active_placard.short_label().to_string(),
        risk_composite: active_comp,
        risk_pml90: active_pml90_01,
        risk_ari: active_ari_risk,
        risk_isolation: active_iso_risk,
    });

    for b in &world_index.buildings {
        if b.is_unlocated() {
            continue;
        }
        let mm = SlippyMapState::lat_lon_to_world_mm(origin_lat_lon, b.lat, b.lon);
        let ex = (mm[0] * 1e-3) as f32;
        let ny = (mm[1] * 1e-3) as f32;
        if ex.hypot(ny) > 650.0 || ex.hypot(ny) < 1.5 {
            continue;
        }
        let b_placard = shm_opt
            .and_then(|s| s.district_placards.iter().find(|d| d.rel_path == b.rel_path))
            .map(|d| d.placard)
            .unwrap_or(if b.min_ari < 0.68 || b.max_dcr > 1.0 {
                Atc20Placard::UnsafeRed
            } else if b.min_ari < 0.82 || b.max_dcr > 0.85 {
                Atc20Placard::RestrictedYellow
            } else {
                Atc20Placard::InspectedGreen
            });
        let b_ari_risk = ((1.0 - b.min_ari).clamp(0.0, 1.0) * 0.65
            + (b.max_dcr / 1.35).clamp(0.0, 1.0) * 0.35) as f32;
        let b_pml90 = (b.total_height_m.max(8.0) * 18_500.0)
            * (1.35 - b.min_ari).clamp(0.25, 1.6);
        let b_pml90_01 = ((b_pml90 / 850_000.0).clamp(0.05, 1.0)) as f32;
        let cordon_prox = evac_opt
            .map(|e| {
                if e.cordons.iter().any(|c| {
                    let dx = c.center_enu_m[0] as f32 - ex;
                    let dy = c.center_enu_m[1] as f32 - ny;
                    dx.hypot(dy) <= (c.cordon_radius_m as f32 + 12.0)
                }) {
                    0.82
                } else {
                    evac_isolation_global * 0.65
                }
            })
            .unwrap_or_else(|| placard_severity_01(b_placard) * 0.72);
        let b_iso = (cordon_prox * 0.65 + placard_severity_01(b_placard) * 0.35).clamp(0.05, 0.99);
        let b_comp = (b_ari_risk * 0.36
            + placard_severity_01(b_placard) * 0.22
            + b_pml90_01 * 0.18
            + b_iso * 0.14
            + liq_boost * 0.75)
            .clamp(0.05, 0.99);
        sources.push(BuildingRiskSource {
            name: b.name.clone(),
            rel_path: b.rel_path.clone(),
            lat: b.lat,
            lon: b.lon,
            east_m: ex,
            north_m: ny,
            min_ari: b.min_ari,
            max_dcr: b.max_dcr,
            pml90_usd: b_pml90,
            atc20_tag: b_placard.short_label().to_string(),
            risk_composite: b_comp,
            risk_pml90: b_pml90_01,
            risk_ari: b_ari_risk,
            risk_isolation: b_iso,
        });
    }

    // Ensure rich surrounding district parcel gradient when fewer than 3 neighbor buildings are geo-pinned
    if sources.len() < 3 {
        let synthetic_offsets = [
            ("North Civic Annex", -52.0_f32, 64.0_f32, 0.74_f64, 0.91_f64, Atc20Placard::RestrictedYellow),
            ("East Riverfront Block", 68.0, 38.0, 0.58, 1.12, Atc20Placard::UnsafeRed),
            ("South Transit Depot", 44.0, -58.0, 0.89, 0.62, Atc20Placard::InspectedGreen),
            ("West Substation Yard", -64.0, -42.0, 0.79, 0.81, Atc20Placard::RestrictedYellow),
        ];
        for (name, ex, ny, ari, dcr, plac) in synthetic_offsets {
            let ll = SlippyMapState::world_mm_to_lat_lon(
                origin_lat_lon,
                [ex as f64 * 1000.0, ny as f64 * 1000.0],
            );
            let s_ari = ((1.0 - ari) * 0.65 + (dcr / 1.35) * 0.35) as f32;
            let s_pml = 320_000.0 * (1.3 - ari).clamp(0.3, 1.4);
            let s_pml01 = ((s_pml / 850_000.0).clamp(0.08, 1.0)) as f32;
            let s_iso = (placard_severity_01(plac) * 0.60 + evac_isolation_global * 0.40).clamp(0.08, 0.96);
            let s_comp = (s_ari * 0.36
                + placard_severity_01(plac) * 0.22
                + s_pml01 * 0.18
                + s_iso * 0.14
                + liq_boost * 0.75)
                .clamp(0.08, 0.98);
            sources.push(BuildingRiskSource {
                name: name.to_string(),
                rel_path: format!("synthetic/{name}"),
                lat: ll.0,
                lon: ll.1,
                east_m: ex,
                north_m: ny,
                min_ari: ari,
                max_dcr: dcr,
                pml90_usd: s_pml,
                atc20_tag: plac.short_label().to_string(),
                risk_composite: s_comp,
                risk_pml90: s_pml01,
                risk_ari: s_ari,
                risk_isolation: s_iso,
            });
        }
    }

    let max_coord = sources
        .iter()
        .map(|s| s.east_m.abs().max(s.north_m.abs()))
        .fold(95.0_f32, f32::max);
    let domain_half_m = (max_coord + 45.0).clamp(110.0, 260.0);
    let grid_n = config.risk_grid_resolution.clamp(10, 32);
    let step_m = (2.0 * domain_half_m) / (grid_n as f32);
    let half_step_m = step_m * 0.5;
    let r0_sq = (step_m * 1.45).powi(2);

    let datum_z = active_site_tin
        .map(|tin| tin.sample_elevation_and_slope(0.0, 0.0).0)
        .unwrap_or(0.0);

    let metric_val = |s: &BuildingRiskSource| -> f32 {
        match config.risk_metric {
            DistrictRiskMetric::CompositeRisk => s.risk_composite,
            DistrictRiskMetric::McPml90Loss => s.risk_pml90,
            DistrictRiskMetric::StructuralAri => s.risk_ari,
            DistrictRiskMetric::EvacuationIsolation => s.risk_isolation,
        }
    };

    let mut cells = Vec::with_capacity(grid_n * grid_n);
    let mut sum_risk = 0.0_f32;
    let mut peak_risk = 0.0_f32;
    let mut high_area_cells = 0usize;
    let (mut low_count, mut moderate_count, mut high_count, mut critical_count) =
        (0usize, 0usize, 0usize, 0usize);

    for iy in 0..grid_n {
        let cy_m = -domain_half_m + (iy as f32 + 0.5) * step_m;
        for ix in 0..grid_n {
            let cx_m = -domain_half_m + (ix as f32 + 0.5) * step_m;
            let mut num = 0.0_f32;
            let mut den = 0.0_f32;
            for s in &sources {
                let dx = cx_m - s.east_m;
                let dy = cy_m - s.north_m;
                let w = 1.0_f32 / (dx * dx + dy * dy + r0_sq);
                num += w * metric_val(s);
                den += w;
            }
            let risk_01 = if den > 1e-8 {
                (num / den).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let elev_m = active_site_tin
                .map(|tin| tin.sample_elevation_and_slope(cx_m, cy_m).0 - datum_z)
                .unwrap_or(0.0);

            sum_risk += risk_01;
            peak_risk = peak_risk.max(risk_01);
            if risk_01 >= 0.65 {
                high_area_cells += 1;
            }
            if risk_01 < 0.35 {
                low_count += 1;
            } else if risk_01 < 0.60 {
                moderate_count += 1;
            } else if risk_01 < 0.80 {
                high_count += 1;
            } else {
                critical_count += 1;
            }

            cells.push(DistrictRiskGridCell {
                ix,
                iy,
                cx_m,
                cy_m,
                half_step_m,
                elev_m,
                risk_01,
            });
        }
    }

    let mut hotspots: Vec<DistrictRiskHotspot> = sources
        .iter()
        .map(|s| {
            let r_score = metric_val(s);
            let raw_val = match config.risk_metric {
                DistrictRiskMetric::CompositeRisk => (r_score * 100.0) as f64,
                DistrictRiskMetric::McPml90Loss => s.pml90_usd * 0.001,
                DistrictRiskMetric::StructuralAri => (1.0 - s.min_ari).clamp(0.0, 1.0) * 100.0,
                DistrictRiskMetric::EvacuationIsolation => (s.risk_isolation * 100.0) as f64,
            };
            DistrictRiskHotspot {
                name: s.name.clone(),
                rel_path: s.rel_path.clone(),
                lat: s.lat,
                lon: s.lon,
                east_m: s.east_m,
                north_m: s.north_m,
                risk_score: r_score,
                metric_value: raw_val,
                min_ari: s.min_ari,
                max_dcr: s.max_dcr,
                pml90_usd: s.pml90_usd,
                atc20_tag: s.atc20_tag.clone(),
            }
        })
        .collect();
    hotspots.sort_by(|a, b| b.risk_score.total_cmp(&a.risk_score));

    let total_cells = cells.len().max(1);
    let mean_risk_01 = sum_risk / (total_cells as f32);
    let high_risk_area_pct = (high_area_cells as f32 / total_cells as f32) * 100.0;

    let mut h = config.content_revision_hash();
    h ^= (sources.len() as u64).wrapping_mul(1_000_003);
    h ^= ((mean_risk_01 * 10_000.0).round() as u64).wrapping_mul(2_654_435_761);
    h ^= ((peak_risk * 10_000.0).round() as u64).wrapping_mul(97);

    let summary = format!(
        "{}: Mean {:.0}% • Peak {:.0}% • High-Risk Area {:.1}% across {} parcels",
        config.risk_metric.short_label(),
        mean_risk_01 * 100.0,
        peak_risk * 100.0,
        high_risk_area_pct,
        sources.len(),
    );

    DistrictRiskFieldReport {
        metric: config.risk_metric,
        grid_n,
        domain_half_m,
        cells,
        hotspots,
        mean_risk_01,
        peak_risk_01: peak_risk,
        high_risk_area_pct,
        low_count,
        moderate_count,
        high_count,
        critical_count,
        evaluated_buildings: sources.len(),
        content_hash: h,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::IfcxStage;

    #[test]
    fn phase80_ifcx_signed_checkpoints_three_way_merge_and_tamper_seal() {
        let proj = BimProject::sample_bimcraft_building();
        let base_doc = IfcxDocument::from_bim_project(&proj, Some((-6.2088, 106.8456, 12.0)), 0.0, None, None, None);

        let mut collab = IfcxCollabState::default();
        let kpi1 = IfcxCheckpointKpi {
            min_ari: 0.74,
            max_dcr_pct: 92.0,
            atc20_placard: "YELLOW".to_string(),
            mc_pml90_usd: 215_000.0,
            evacuation_conn_pct: 88.0,
            node_count: base_doc.data.len(),
        };
        collab.create_checkpoint("Baseline Design", CollabDiscipline::Structural, base_doc.clone(), kpi1.clone());

        // Second chained checkpoint
        let mut local_doc = base_doc.clone();
        if let Some(node) = local_doc
            .data
            .iter_mut()
            .find(|n| n.attributes.contains_key(ns::BSI_PROP))
        {
            let mut prop = node.attributes.get(ns::BSI_PROP).cloned().unwrap_or_else(|| json!({}));
            if let Some(obj) = prop.as_object_mut() {
                obj.insert("peer_review_note".to_string(), json!("Local structural stiffener added"));
            }
            node.attributes.insert(ns::BSI_PROP.to_string(), prop);
        }
        collab.create_checkpoint("Local Ductility Update", CollabDiscipline::Structural, local_doc.clone(), kpi1);
        assert_eq!(collab.checkpoints.len(), 2);
        assert_eq!(collab.verify_chain(), (true, 0));

        // Approval sign-off + chain preservation
        let sign_msg = collab.approve_active_checkpoint("Ir. Ardi, PE").expect("approve");
        assert!(sign_msg.contains("CP-002"));
        assert_eq!(collab.checkpoints[1].approval_status, CheckpointApprovalStatus::Approved);
        assert_eq!(collab.verify_chain(), (true, 0));

        // Governance IFCX overlay + classification check
        let gov_overlay = collab.build_governance_seal_overlay(&proj).expect("gov overlay");
        let composed = IfcxStage::compose_with_governance_seal_overlay(&local_doc, &gov_overlay);
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(ns::EG_GOVERNANCE_SEAL)));

        // Bundle export / verified round-trip
        let bundle_json = collab.export_bundle_json(&proj.project_name).expect("bundle json");
        let mut restored = IfcxCollabState::default();
        let imported = restored.import_bundle_json(&bundle_json).expect("verified import");
        assert_eq!(imported, 2);
        assert_eq!(restored.checkpoints[1].approval_status, CheckpointApprovalStatus::Approved);

        // Tamper detection on snapshot mutation
        let mut tampered = collab.checkpoints.clone();
        tampered[0].label = "FORGED".to_string();
        if let Some(first_node) = tampered[0].snapshot.data.first_mut() {
            first_node.path.push_str("_tampered");
        }
        let (ok, bad) = verify_checkpoint_chain(&tampered);
        assert!(!ok);
        assert!(bad >= 1);

        // Bundle tamper detection
        let altered_json = bundle_json.replace("CP-002", "CP-999");
        assert!(IfcxCollabBundle::from_verified_json(&altered_json).is_err());

        // Three-way merge with auto-merged nodes + conflict resolution
        let peer_doc = synthesize_peer_branch_doc(&base_doc, CollabDiscipline::PeerReview);
        let mut merge = three_way_merge_ifcx(
            &base_doc,
            &local_doc,
            &peer_doc,
            "CP-001",
            "Local",
            "PeerReview",
        );
        assert!(merge.auto_merged_nodes >= 1);
        assert!(!merge.conflicts.is_empty());
        assert!(merge
            .merged_doc
            .data
            .iter()
            .any(|n| n.path == "/earth/collab/peer_peerreview"));

        let sha_before = merge.merged_sha256.clone();
        apply_conflict_resolution(&mut merge, 0, ConflictResolution::KeepBothLayered);
        assert_ne!(merge.merged_sha256, sha_before);
        assert_eq!(merge.conflicts[0].resolution, ConflictResolution::KeepBothLayered);

        // District Risk Field across all 4 metrics
        let world_idx = WorldSpatialIndex::default();
        for metric in DistrictRiskMetric::all() {
            let cfg = IfcxCollabConfig {
                risk_metric: *metric,
                risk_grid_resolution: 14,
                risk_heatmap_visible: true,
                ..Default::default()
            };
            let report = evaluate_district_risk_field(
                &cfg,
                (-6.2088, 106.8456),
                &proj,
                &[],
                &world_idx,
                None,
                None,
                None,
                None,
                None,
            );
            assert_eq!(report.grid_n, 14);
            assert_eq!(report.cells.len(), 196);
            assert!(report.evaluated_buildings >= 3);
            assert!(!report.hotspots.is_empty());
            assert!(report.peak_risk_01 >= report.mean_risk_01);
        }
    }

    #[test]
    fn phase81_collab_relay_loopback_and_peer_delta_merge() {
        use crate::core::web_bridge::{
            collab_relay_connect, collab_relay_disconnect, collab_relay_inject_event,
            collab_relay_poll_events, collab_relay_publish_json, collab_relay_snapshot,
            CollabRelayEvent,
        };

        let proj = BimProject::sample_bimcraft_building();
        let base_doc = IfcxDocument::from_bim_project(&proj, None, 0.0, None, None, None);
        let peer_doc = synthesize_peer_branch_doc(&base_doc, CollabDiscipline::Geotechnical);
        let peer_json = peer_doc.to_json_pretty();

        let status = collab_relay_connect("wss://relay.eigengraph.net/test").expect("relay connect");
        assert!(status.contains("wss://relay.eigengraph.net/test"));
        assert!(collab_relay_snapshot().0);

        let pub_msg = collab_relay_publish_json(&peer_json).expect("publish");
        assert!(pub_msg.contains("byte"));

        collab_relay_inject_event(CollabRelayEvent {
            peer_author: "Dr. Sinta, GE".to_string(),
            discipline: "Geotech".to_string(),
            checkpoint_id: "CP-PEER-01".to_string(),
            payload_json: peer_json,
        });
        let events = collab_relay_poll_events();
        assert_eq!(events.len(), 1);
        let parsed = IfcxDocument::parse(&events[0].payload_json).expect("parse relay ifcx");

        let mut collab = IfcxCollabState::default();
        let merge_msg = collab.ingest_relay_peer_doc(
            &base_doc,
            &base_doc,
            &events[0].peer_author,
            CollabDiscipline::Geotechnical,
            &parsed,
        );
        assert!(merge_msg.contains("3-Way IFCX Merge"));
        assert_eq!(collab.relay_synced_count, 1);
        collab_relay_disconnect();
        assert!(!collab_relay_snapshot().0);
    }
}
