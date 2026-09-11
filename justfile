et frontend="tauri":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{frontend}}" in
        tauri) cd bin/desktop/src-tauri && cargo test --lib export_bindings ;;
        swift) bash bin/macos/build.sh bindings ;;
        *) echo "Unknown frontend: {{frontend}}" >&2; exit 2 ;;
    esac

t:
    cd bin/desktop && npm run tauri dev

m mode="debug": (mb mode)
    open bin/macos/.build/Pitpls.app

mb mode="debug":
    bash bin/macos/build.sh {{quote(mode)}}
