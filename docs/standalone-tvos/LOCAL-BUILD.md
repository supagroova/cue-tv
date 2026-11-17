# Local Apple TV build

Source: https://github.com/rclaycock/cue-tv (main, retrieved 16 November 2026).
This is an extracted Apple TV target and its shared assets, not a full Git checkout.

Open `apple-tv/SittingRoomTV.xcodeproj` in Xcode. The app is currently named **TV** by the upstream project.

The local signing configuration has an empty DEVELOPMENT_TEAM. For device installation, select your own team in Signing & Capabilities and choose an available personal bundle identifier. Pair your Apple TV in Xcode's Devices and Simulators window, then select it as the run destination.

`apple-tv/TVConfig.plist` contains upstream dummy network values, not a working household backend. The app requires the Cue TV Android service for pairing, catalogues, library and playback controls, and opens a separately installed Stremio app for video playback. DietPi's Sonarr, Transmission and SMB services do not implement that backend. Standalone Apple TV support is not implemented in this repository snapshot.

No changes have been made to your Apple TV or Raspberry Pi.

To regenerate the Xcode project with XcodeGen:

```sh
xcodegen generate --spec apple-tv/project.yml
```

To build without device signing:

```sh
xcodebuild -project apple-tv/SittingRoomTV.xcodeproj -scheme SittingRoomTV -configuration Debug -sdk appletvos -destination 'generic/platform=tvOS' -derivedDataPath build CODE_SIGNING_ALLOWED=NO build
```

An unsigned build cannot be installed directly on an Apple TV. Xcode must sign and provision a device build first.
