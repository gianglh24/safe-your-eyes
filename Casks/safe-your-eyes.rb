cask "safe-your-eyes" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.0"
  sha256 arm:   "25d880443f6de17bdd3bd2bc3d7177128fee12724e893e638fd6e1ca5cdbd07f",
         intel: "6f602e1f7b2fb3cc83b3d6d51e90833cfdc44a6e2d175ded502eb3c65bff209f"

  url "https://github.com/tunglt1810/safe-your-eyes/releases/download/v#{version}/safe-your-eyes-macos-#{arch}"
  name "Safe Your Eyes"
  desc "Lightweight cross-platform Rust eye-protection desktop app"
  homepage "https://tunglt1810.github.io/safe-your-eyes/"

  binary "safe-your-eyes-macos-#{arch}", target: "safe-your-eyes"

  zap trash: [
    "~/Library/Application Support/safe-your-eyes",
    "~/Library/Caches/safe-your-eyes",
    "~/Library/LaunchAgents/io.github.safeyoureyes.SafeYourEyes.plist",
  ]

  caveats <<~EOS
    Start the background daemon with:
      safe-your-eyes

    Open the Cozy Settings window with:
      safe-your-eyes --settings
  EOS
end
