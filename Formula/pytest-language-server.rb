class PytestLanguageServer < Formula
  desc "Blazingly fast Language Server Protocol implementation for pytest"
  homepage "https://github.com/bellini666/pytest-language-server"
  version "0.19.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.0/pytest-language-server-aarch64-apple-darwin"
      sha256 "def264b3aa573ac143ad4f5c35bbb6f25c12d75f8607b0c4517e203b39cb8a6f"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.0/pytest-language-server-x86_64-apple-darwin"
      sha256 "c9b53a08164727dcf42ca6efe4d2eaa96d08141f5f63f011d034ee2d9c610d0a"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.0/pytest-language-server-aarch64-unknown-linux-gnu"
      sha256 "d297871f6d707cd221547ff2b2d901b3d0538bc5e30ded29bb3bffcd403a5e8f"
    else
      url "https://github.com/bellini666/pytest-language-server/releases/download/v0.19.0/pytest-language-server-x86_64-unknown-linux-gnu"
      sha256 "5be820b83191f9f3500e40505beb5db012030238e8aa2294b79cb3e388a4f6fe"
    end
  end

  def install
    bin.install cached_download => "pytest-language-server"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pytest-language-server --version")
  end
end
