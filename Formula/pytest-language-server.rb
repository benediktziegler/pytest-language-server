class PytestLanguageServer < Formula
  desc "Blazingly fast Language Server Protocol implementation for pytest"
  homepage "https://github.com/bellini666/pytest-language-server"
  version "0.18.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.18.0/pytest-language-server-aarch64-apple-darwin"
      sha256 "0781bec560f6cd4edd63ddbd40486ccd34da4b06920d97752907b31dbb4ea7fd"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.18.0/pytest-language-server-x86_64-apple-darwin"
      sha256 "e742ea21adac480d0b789bdada2b5e53ac7df307edfe00b1517a17df846674e8"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.18.0/pytest-language-server-aarch64-unknown-linux-gnu"
      sha256 "d15e9410a45e49292ba0d665f0b645da73c9cd89381a1e96f1f6273b143ccec0"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.18.0/pytest-language-server-x86_64-unknown-linux-gnu"
      sha256 "c3e5e5616e246649d4f1e56cd48c495963803990956c7fcf5d1ea1a874eeb514"
    end
  end

  def install
    bin.install cached_download => "pytest-language-server"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pytest-language-server --version")
  end
end
