# Sharing and privacy checks

This snapshot excludes the live configuration and generated artefacts. Household endpoints in examples/tests were replaced with dummy private-network values, and wake-packet tests now use a dummy locally administered MAC. The original installed source and configuration were not rewritten to produce this share copy.

`privacy-fixtures.json` records exact reviewed synthetic lines by path, rule and SHA-256. The generic privacy scanner filters a line only when all three match. It applies the same checks to history. These exceptions do not apply to gitleaks or forbidden-file detection. A new or modified line is checked normally. The author business contact in portable information pages is an example address.

Apple bundle identifiers retain the original app identity as source metadata; each independent developer must configure their own signing. No certificates, provisioning profiles or private keys are included. Existing source licences and upstream attribution are retained.

Do not commit private account URLs, device logs, screenshots, recordings, cloud credentials or local signing/configuration. Do not add broad test-directory exclusions to the scanner.
