class PytestLanguageServer < Formula
  desc "Blazingly fast Language Server Protocol implementation for pytest"
  homepage "https://github.com/bellini666/pytest-language-server"
  version "0.20.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.20.0/pytest-language-server-aarch64-apple-darwin"
      sha256 "f219865c770b7b6cb82b358ec60c5d5d5c6fa545090403f4f0a59da909f98636"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.20.0/pytest-language-server-x86_64-apple-darwin"
      sha256 "b608b4c61a15ee9a06ce62d3f6be33f5fcf4516265e81f213055cb6f3a7db371"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.20.0/pytest-language-server-aarch64-unknown-linux-gnu"
      sha256 "cfced58b2e5eb850b2ebc1a5b02de96222b570374e1de6962315c888dbd9d250"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.20.0/pytest-language-server-x86_64-unknown-linux-gnu"
      sha256 "3dadfa4b3a952c9b250b77d6da23c77e35ed09291f2c3aeb2e1d466bd73088b3"
    end
  end

  def install
    bin.install cached_download => "pytest-language-server"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pytest-language-server --version")
  end
end
