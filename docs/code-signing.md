# ORBIT Authenticode Code-Signing Guide

This document establishes the official Authenticode code-signing procedures for ORBIT release binaries on Windows.

## 1. Overview & Threat Model

Unsigned desktop executables on Windows face immediate trust penalties:
- **Windows SmartScreen**: Triggers prominent "Windows protected your PC" warnings on launch until sufficient reputation is acquired.
- **Antivirus Heuristics**: Capabilities such as global hotkeys (`RegisterHotKey`), desktop background capture, and memory trimming can trigger false-positive detections on unsigned binaries.
- **Supply Chain Integrity**: Code signing with an RFC 3161 timestamp guarantees binary integrity and proves authorship.

All production binaries (`orbit.exe` and `ORBIT_<version>_x64-setup.exe`) should be signed with an Authenticode certificate using SHA-256 digest and timestamping.

---

## 2. Recommended Signing Solutions

### Option A: SignPath Foundation (Recommended for Open Source)

SignPath provides free code signing for qualifying open-source software via the SignPath Foundation.

1. **SignPath Account Setup**:
   - Register the project at [SignPath.io](https://about.signpath.io/).
   - Apply for the Open Source program (requires a public GitHub repository with an OSI-approved license).
2. **Configure Signing Policy**:
   - Create a project named `ORBIT`.
   - Define a signing policy (e.g. `release-signing`) restricted to tagged releases on the main branch.
3. **GitHub Actions Integration**:
   - Store the API token in GitHub Secrets (`SIGNPATH_API_TOKEN`).
   - Use the official SignPath action in your release workflow:

```yaml
- name: Sign Release Executables
  uses: SignPath/github-action-submit-signing-request@v1
  with:
    api-token: ${{ secrets.SIGNPATH_API_TOKEN }}
    organization-id: ${{ secrets.SIGNPATH_ORG_ID }}
    project-slug: 'orbit'
    signing-policy-slug: 'release-signing'
    artifact-path: 'src-tauri/target/release/orbit.exe'
    output-artifact-directory: 'release/'
    wait-for-completion: true
```

---

### Option B: Microsoft Azure Trusted Signing (Cloud-Native)

Azure Trusted Signing (formerly Azure Code Signing) provides managed FIPS 140-2 Level 3 HSM keys without physical hardware dongles.

1. **Prerequisites**:
   - Azure Subscription with identity validation for individual or organization.
   - A **Trusted Signing Account** in Azure Portal with a **Certificate Profile** (Public Trust).
2. **Permissions**:
   - Grant the signing workflow service principal the `Trusted Signing Certificate Profile Signer` role.
3. **GitHub Actions Integration**:

```yaml
- name: Azure Trusted Signing
  uses: azure/trusted-signing-action@v0.5.1
  with:
    azure-tenant-id: ${{ secrets.AZURE_TENANT_ID }}
    azure-client-id: ${{ secrets.AZURE_CLIENT_ID }}
    azure-client-secret: ${{ secrets.AZURE_CLIENT_SECRET }}
    endpoint: 'https://weu.codesigning.azure.net/'
    trusted-signing-account-name: 'orbit-signing'
    certificate-profile-name: 'orbit-release'
    files: |
      release/orbit.exe
      release/ORBIT_*.exe
    file-digest: 'SHA256'
    timestamp-rfc3161: 'http://timestamp.acs.microsoft.com'
    timestamp-digest: 'SHA256'
```

---

## 3. Local Developer Testing (Self-Signed Certificates)

For local development or testing without a cloud certificate:

1. **Generate a Self-Signed Test Certificate**:
   Run PowerShell as Administrator:

   ```powershell
   $cert = New-SelfSignedCertificate `
       -Type CodeSigningCert `
       -Subject "CN=ORBIT Island Dev Test" `
       -CertStoreLocation "Cert:\CurrentUser\My" `
       -HashAlgorithm SHA256

   # Trust the certificate locally
   Export-Certificate -Cert $cert -FilePath "$env:TEMP\orbit_dev.cer"
   Import-Certificate -FilePath "$env:TEMP\orbit_dev.cer" -CertStoreLocation "Cert:\CurrentUser\Root"
   ```

2. **Sign Local Binaries**:

   ```powershell
   & "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe" sign `
       /fd SHA256 `
       /a `
       /s My `
       /n "ORBIT Island Dev Test" `
       /tr http://timestamp.digicert.com `
       /td SHA256 `
       src-tauri\target\release\orbit.exe
   ```

---

## 4. Verification

Verify the digital signature using PowerShell:

```powershell
Get-AuthenticodeSignature release\orbit.exe | Format-List
```

Expected output for a valid signed binary:
- `Status`: `Valid`
- `StatusMessage`: `Signature verified.`
- `SignerCertificate`: Valid certificate chain.
- `TimeStamperCertificate`: Valid RFC 3161 timestamp signature.
