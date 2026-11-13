# Contributions and third-party materials

Catch's new original code is licensed under MIT. This notice does not relicense independently licensed materials or expand any third-party grant.

## Project history and authorized contributions

Catch began as an extension of [pearfish223/Floatodo](https://github.com/pearfish223/Floatodo). Version 2.0 replaces the identified inherited todo/history implementation, configuration store, application/window assembly, preload bridges and mixed document shells. The old upstream source and Git history are not included in this repository. Product inspiration and commonplace Electron API usage are acknowledged; this statement is a source review, not a claim of legal certainty.

Catch includes interface and interaction contributions by jianxunbi (suppermaker), used with permission.

## Bundled materials

| Material | Creator/source | License |
|---|---|---|
| Manrope font | Font authors credited in the bundled OFL file | SIL Open Font License 1.1, `assets/fonts/Manrope-OFL.txt` |
| Fraunces font | Font authors credited in the bundled OFL file | SIL Open Font License 1.1, `assets/fonts/Fraunces-OFL.txt` |
| Growing Plant sprite adaptation | Dean Cole, [Growing Plant](https://lottiefiles.com/free-animation/growing-plant-p4PvMFxRbS) | Lottie Simple License (FL 9.13.21), `assets/sprout/LICENSE.txt`; source/hash details in `assets/sprout/SOURCE.json` |
| Electron runtime | Electron contributors | MIT; `LICENSE.electron.txt` in packaged builds |
| Chromium and runtime components | Respective contributors | Packaged `LICENSES.chromium.html` |

The plant sprite remains under the Lottie Simple License, including its adaptation. Font and animation files are not relicensed under Catch's MIT license. The application icon is generated from Catch's check mark by `tools/icon.js` and is included under MIT.

Development tools (Electron Builder, esbuild, TypeScript and others) retain the licenses in their packages. They are not runtime npm dependencies shipped as application code. The renderer bundle's generated legal comments must not be removed from a distributed build.
