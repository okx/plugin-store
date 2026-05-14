from pathlib import Path
import shutil


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    targets = []
    pytest_cache = ROOT / ".pytest_cache"
    if pytest_cache.exists():
        targets.append(pytest_cache)
    targets.extend(ROOT.rglob("__pycache__"))

    removed = 0
    for target in targets:
        resolved = target.resolve()
        if ROOT.resolve() not in (resolved, *resolved.parents):
            raise RuntimeError(f"Refusing to remove outside plugin root: {resolved}")
        shutil.rmtree(resolved, ignore_errors=True)
        removed += 1

    print(f"Removed {removed} cache directories under {ROOT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
