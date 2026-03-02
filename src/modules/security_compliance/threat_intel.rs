#![allow(dead_code)]
// Threat Intelligence Integration
use anyhow::Result;

use super::{
    Effort, Priority, RecommendationCategory, SecurityRecommendation, ThreatAssessment, ThreatLevel,
};

/// Integrates with threat intelligence feeds
#[derive(Default)]
pub struct ThreatIntelligence {
    feeds_enabled: bool,
}

impl ThreatIntelligence {
    pub fn new() -> Result<Self> {
        Ok(Self {
            feeds_enabled: false, // No feeds are actually configured
        })
    }

    pub async fn assess(&self, indicator: &str) -> Result<ThreatAssessment> {
        tracing::debug!("Assessing threat indicator: {}", indicator);

        if !self.feeds_enabled {
            tracing::warn!(
                indicator = %indicator,
                "Threat assessment requested but no feeds are configured. \
                 Returning 'not_assessed' with zero confidence. \
                 Configure threat intel feeds (VirusTotal, AlienVault OTX, Abuse.ch, MISP) \
                 for real assessments."
            );

            return Ok(ThreatAssessment {
                indicator: indicator.to_string(),
                threat_level: ThreatLevel::Clean, // Not assessed, not "clean"
                categories: vec![],
                sources: vec!["not_assessed: no threat intelligence feeds configured".to_string()],
                first_seen: None,
                last_seen: None,
                confidence: 0.0,
            });
        }

        // In real implementation: query threat intel APIs
        // - VirusTotal
        // - AlienVault OTX
        // - Abuse.ch
        // - MISP

        Ok(ThreatAssessment {
            indicator: indicator.to_string(),
            threat_level: ThreatLevel::Clean,
            categories: vec![],
            sources: vec!["not_assessed: feed integration not implemented".to_string()],
            first_seen: None,
            last_seen: None,
            confidence: 0.0,
        })
    }

    pub async fn get_recommendations(&self) -> Result<Vec<SecurityRecommendation>> {
        Ok(vec![SecurityRecommendation {
            priority: Priority::P3Medium,
            category: RecommendationCategory::ThreatMitigation,
            title: "Enable threat intelligence feeds".to_string(),
            description: "Integrate with external threat feeds for real-time protection"
                .to_string(),
            impact: "Proactive threat detection".to_string(),
            effort: Effort::Medium,
            auto_applicable: false,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_intelligence_creation() {
        let ti = ThreatIntelligence::new();
        assert!(ti.is_ok());
    }

    #[test]
    fn test_threat_intelligence_default() {
        let ti = ThreatIntelligence::default();
        assert!(!ti.feeds_enabled);
    }

    #[tokio::test]
    async fn test_assess_returns_clean_without_feeds() {
        let ti = ThreatIntelligence::new().unwrap();
        let assessment = ti.assess("192.168.1.1").await.unwrap();
        assert_eq!(assessment.indicator, "192.168.1.1");
        assert_eq!(assessment.threat_level, ThreatLevel::Clean);
        assert_eq!(assessment.confidence, 0.0);
        assert!(assessment.categories.is_empty());
        assert!(assessment.first_seen.is_none());
        assert!(assessment.last_seen.is_none());
    }

    #[tokio::test]
    async fn test_assess_domain_indicator() {
        let ti = ThreatIntelligence::new().unwrap();
        let assessment = ti.assess("malicious.example.com").await.unwrap();
        assert_eq!(assessment.indicator, "malicious.example.com");
        assert_eq!(assessment.threat_level, ThreatLevel::Clean);
    }

    #[tokio::test]
    async fn test_assess_sources_indicate_not_assessed() {
        let ti = ThreatIntelligence::new().unwrap();
        let assessment = ti.assess("10.0.0.1").await.unwrap();
        assert!(!assessment.sources.is_empty());
        assert!(
            assessment.sources[0].contains("not_assessed"),
            "Source should indicate feeds are not configured"
        );
    }

    #[tokio::test]
    async fn test_get_recommendations() {
        let ti = ThreatIntelligence::new().unwrap();
        let recs = ti.get_recommendations().await.unwrap();
        assert!(!recs.is_empty());
        assert_eq!(recs[0].category, RecommendationCategory::ThreatMitigation);
        assert_eq!(recs[0].priority, Priority::P3Medium);
    }
}
