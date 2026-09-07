import json
from pathlib import Path

from strava_media_archive.core import MediaItem, download_items, make_archive


class FakeClient:
    def fetch(self, url):
        return ("content:" + url).encode()


def test_download_and_manifest(tmp_path):
    items = [MediaItem("https://cdn/a.jpg", "image", "source", "a.jpg")]
    out = tmp_path / "folder"
    download_items(FakeClient(), items, out)
    assert (out / "a.jpg").read_bytes() == b"content:https://cdn/a.jpg"
    assert json.loads((out / "manifest.json").read_text())[0]["kind"] == "image"


def test_make_archive(tmp_path):
    folder = tmp_path / "folder"
    folder.mkdir()
    (folder / "a.txt").write_text("ok")
    archive = make_archive(folder, tmp_path / "out.zip")
    import zipfile
    with zipfile.ZipFile(archive) as zf:
        assert zf.read("a.txt") == b"ok"
