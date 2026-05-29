use sdkwork_agent_kernel::{
    KernelConformanceCase, KernelConformanceCaseStatus, KernelConformanceReport,
    AGENT_KERNEL_SPEC_VERSION, KERNEL_CONFORMANCE_REPORT_SCHEMA,
};

#[test]
fn conformance_report_tracks_profile_versions_counts_required_skips_and_failures() {
    let report = KernelConformanceReport::new(
        "report.runtime.1",
        "runtime-core",
        "implementation.runtime.local",
        "0.1.0",
    )
    .with_spec_version(AGENT_KERNEL_SPEC_VERSION)
    .with_test_suite_version("suite.0.1.0")
    .with_security_profile("local-trusted")
    .with_required_capability("runtime.bootstrap")
    .with_required_capability("policy.evaluate")
    .add_case(
        KernelConformanceCase::passed("case.manifest.valid", "manifest validates")
            .required()
            .for_capability("runtime.bootstrap"),
    )
    .add_case(
        KernelConformanceCase::skipped("case.model.streaming", "model streaming not claimed")
            .with_skip_reason("optional capability not claimed"),
    )
    .add_case(
        KernelConformanceCase::failed("case.policy.denial", "policy denial did not fail closed")
            .required()
            .for_capability("policy.evaluate"),
    );

    assert_eq!(report.report_id, "report.runtime.1");
    assert_eq!(report.profile_id, "runtime-core");
    assert_eq!(report.implementation_id, "implementation.runtime.local");
    assert_eq!(report.implementation_version, "0.1.0");
    assert_eq!(
        report.spec_version.as_deref(),
        Some(AGENT_KERNEL_SPEC_VERSION)
    );
    assert_eq!(report.test_suite_version.as_deref(), Some("suite.0.1.0"));
    assert_eq!(report.security_profile.as_deref(), Some("local-trusted"));
    assert_eq!(
        report.required_capabilities,
        ["runtime.bootstrap", "policy.evaluate"]
    );
    assert_eq!(report.passed_count(), 1);
    assert_eq!(report.failed_count(), 1);
    assert_eq!(report.skipped_count(), 1);
    assert!(!report.is_passed());
    assert_eq!(report.failed_case_ids(), ["case.policy.denial"]);
    assert_eq!(
        report
            .case("case.model.streaming")
            .expect("skip case exists")
            .status,
        KernelConformanceCaseStatus::Skipped
    );

    let required_skip_report = KernelConformanceReport::new(
        "report.runtime.2",
        "runtime-core",
        "implementation.runtime.local",
        "0.1.0",
    )
    .add_case(
        KernelConformanceCase::skipped("case.runtime.cancel", "cancel test unavailable")
            .required()
            .with_skip_reason("runner missing cancellation harness"),
    );

    assert!(!required_skip_report.is_passed());
    assert_eq!(
        required_skip_report.required_skipped_case_ids(),
        ["case.runtime.cancel"]
    );
}

#[test]
fn conformance_report_schema_is_exported_for_standard_runners() {
    assert!(KERNEL_CONFORMANCE_REPORT_SCHEMA.contains("SDKWork Kernel Conformance Report"));
    assert!(KERNEL_CONFORMANCE_REPORT_SCHEMA.contains("kernel_conformance_report"));
    assert!(KERNEL_CONFORMANCE_REPORT_SCHEMA.contains("\"status\""));
}

#[test]
fn conformance_report_deduplicates_required_capabilities_to_match_schema() {
    let report = KernelConformanceReport::new(
        "report.runtime.duplicates",
        "runtime-core",
        "implementation.runtime.local",
        "0.1.0",
    )
    .with_required_capability("model.chat")
    .with_required_capability("model.chat")
    .with_required_capability("policy.evaluate");

    assert_eq!(
        report.required_capabilities,
        ["model.chat", "policy.evaluate"]
    );
}
