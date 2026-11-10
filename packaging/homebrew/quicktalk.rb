# Template for Gitguy268/homebrew-tap Casks/quicktalk.rb. Fill in SHA256_ARM and SHA256_INTEL
# from the release's SHA256SUMS after the build workflow has attached the disk images.
cask "quicktalk" do
  arch arm: "aarch64", intel: "x64"

  version "0.1.0"
  sha256 arm:   "SHA256_ARM",
         intel: "SHA256_INTEL"

  url "https://github.com/Gitguy268/quicktalk/releases/download/v#{version}/QuickTalk_#{version}_#{arch}.dmg"
  name "QuickTalk"
  desc "Local-first dictation and meeting notes"
  homepage "https://github.com/Gitguy268/quicktalk"

  depends_on macos: ">= :sonoma"

  app "QuickTalk.app"

  # The release build is not notarised; clear the download quarantine so it opens.
  postflight do
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/QuickTalk.app"]
  end

  zap trash: [
    "~/Library/Application Support/app.quicktalk.desktop",
    "~/Library/Caches/app.quicktalk.desktop",
    "~/Library/WebKit/app.quicktalk.desktop",
  ]

  caveats <<~EOS
    QuickTalk is not signed by Apple. The cask removes the quarantine flag after install.
    macOS asks for microphone access the first time you dictate.
  EOS
end
