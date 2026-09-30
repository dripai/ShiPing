# ShiPing Microsoft Store MSIX package

`tools/package-store-msix.ps1` packages only ShiPing. It uses `packaging/shiping/AppxManifest.xml`, preserves the existing Store identity, replaces `__PACKAGE_VERSION__` with `X.Y.Z.0`, and rejects a mismatched executable name.

```powershell
.\tools\package-store-msix.ps1 -Product ShiPing -ExecutablePath .\target\release\ShiPing.exe -Version 0.1.34 -OutputDirectory .\release-assets
```

Windows SDK `MakeAppx.exe` is required. Output:

- `ShiPing-0.1.34-windows-x64.msix`: unsigned MSIX.
- `ShiPing-0.1.34-store.msixupload`: Partner Center upload archive.

ShiPing declares `runFullTrust` and has no Windows AI OCR or Windows App Runtime dependency.

Microsoft Store signs accepted submissions. The unsigned package is not a signed sideloading installer. Local packaging checks do not confirm Store acceptance or installation on other Windows versions.

Official references:

- https://learn.microsoft.com/windows/apps/package-and-deploy/choose-distribution-path
- https://learn.microsoft.com/windows/apps/publish/publish-your-app/msix/upload-app-packages
