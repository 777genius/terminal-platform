# Research: Terminal Foundations

**Проверено**: 2026-04-19

Этот файл не про "какой terminal app красивее", а про то, что реально можно взять как **foundation** для новой feature внутри нашего Electron/React продукта.

## Что считаем хорошим foundation

Foundation здесь должен:

- реально встраиваться в React/Electron
- быть пригодным для full PTY terminal, а не fake console UI
- не ломать TUI apps
- давать понятную историю для selection, search, links, resize, scrollback
- не тащить в нас чужой whole app вместо библиотеки

## 1. `wterm`

### Snapshot

- Repo: [`vercel-labs/wterm`](https://github.com/vercel-labs/wterm)
- Stars: `1754`
- Pushed: `2026-04-18`
- NPM: [`@wterm/react 0.1.8`](https://www.npmjs.com/package/@wterm/react), опубликован `2026-04-16`

### Что это такое

`wterm` - web terminal с Zig + WASM core и DOM renderer.

Пакетный стек:

- `@wterm/core` - headless WASM bridge + WebSocket transport
- `@wterm/dom` - DOM renderer
- `@wterm/react` - React component + hook
- `@wterm/just-bash` - in-browser shell
- `@wterm/markdown` - markdown rendering inside terminal

### Что в нём реально сильное

- DOM rendering вместо canvas
- native text selection
- browser find
- accessibility / screen reader story
- clipboard без костылей
- alternate screen, scrollback, auto-resize
- WebSocket transport уже предусмотрен в дизайне

### Что в нём особенно важно для нас

Если новая terminal feature будет жить внутри React UI, то `wterm` даёт самый clean embed story из найденного:

- не надо сразу писать свой React wrapper
- нет ощущения, что ты встраиваешь half-IDE или целый чужой app
- DOM model делает terminal ближе к browser-native UX

### Где риск

⚠️ Проект очень молодой:

- пакет создан только `2026-04-14`
- экосистема пока маленькая
- долгосрочная стабильность ещё не доказана

### Вывод

`wterm` - лучший кандидат, если нужен **современный и реально встраиваемый** terminal foundation для React/Electron.

### Оценка

`🎯 8   🛡️ 5   🧠 5`  
Примерно `2000-4500` строк.

---

## 2. `restty`

### Snapshot

- Repo: [`wiedymi/restty`](https://github.com/wiedymi/restty)
- Stars: `339`
- Pushed: `2026-04-03`
- NPM: [`restty 0.1.35`](https://www.npmjs.com/package/restty), опубликован `2026-04-03`

### Что это такое

`restty` позиционируется как powerful lightweight browser terminal с batteries included.

По README и package:

- `libghostty-vt` как terminal core
- `WebGPU` с `WebGL2` fallback
- `text-shaper` для shaping/raster
- built-in panes
- themes
- ligatures
- selection
- `connectPty("ws://...")`

### Что в нём реально сильное

В отличие от просто renderer-обвязки, `restty` уже ощущается как **terminal stack**, а не как минимальный terminal widget:

- он сам собирает DOM/canvas/input plumbing
- умеет panes
- уже думает про fonts, shaping, themes, ligatures
- имеет high-level API вокруг активной pane

### Что важно для нас

Если хочется собрать terminal feature быстрее и получить более "богатый" foundation, `restty` выглядит сильнее многих альтернатив.

Он ближе к ответу на вопрос:

"как встроить не просто terminal emulator, а уже modern terminal surface?"

### Где риск

⚠️ Всё ещё ранний проект:

- версия `0.1.x`
- автор прямо пишет, что high-level APIs usable now, но API может меняться
- известны edge cases, например kitty image protocol

Ещё важный практический момент:

- build/tooling завязан на `bun`
- React wrapper из коробки нет

### Вывод

`restty` - самый интересный `batteries-included` foundation, если готовы принять свежесть и меньшую зрелость экосистемы.

### Оценка

`🎯 8   🛡️ 6   🧠 6`  
Примерно `2500-5000` строк.

---

## 3. `ghostty-web`

### Snapshot

- Repo: [`coder/ghostty-web`](https://github.com/coder/ghostty-web)
- Stars: `2341`
- Pushed: `2026-04-13`
- NPM latest stable: [`ghostty-web 0.4.0`](https://www.npmjs.com/package/ghostty-web), опубликован `2025-12-09`

### Что это такое

`ghostty-web` - Ghostty terminal core в web через WASM с xterm-compatible API.

Сильные моменты:

- proper VT100 implementation
- xterm-like API migration path
- zero runtime deps
- около `400KB` WASM bundle

### Что важно понять

Это хороший путь не потому что "ещё один xterm clone", а потому что:

- ядро приходит из Ghostty
- это уже battle-tested emulator logic
- библиотека делалась как foundation для `Mux`

### Полезный технический сигнал

Есть интересный нюанс:

- в npm latest stable - `0.4.0`
- в default branch `package.json` repo сейчас видно `0.3.0`

Это не критичный минус, но это reminder, что вокруг пакета ещё есть release churn.

### Где риск

- своего React adapter нет
- higher-level UX почти весь придётся делать самим
- это всё ещё web wrapper вокруг evolving Ghostty stack

### Вывод

`ghostty-web` - хороший lower-level base, если хочется двигаться в сторону `libghostty`, но не лезть сразу в raw native bindings.

### Оценка

`🎯 7   🛡️ 6   🧠 7`  
Примерно `2500-5000` строк.

---

## 4. `floeterm`

### Snapshot

- Repo: [`floegence/floeterm`](https://github.com/floegence/floeterm)
- Stars: `0`
- Pushed: `2026-04-18`
- NPM: [`@floegence/floeterm-terminal-web 0.4.16`](https://www.npmjs.com/package/@floegence/floeterm-terminal-web), опубликован `2026-04-18`

### Что это такое

`floeterm` - не просто renderer, а product-first terminal infrastructure stack:

- `terminal-go` - PTY backend
- `terminal-web` - headless browser/web layer
- reference app

Особенно интересны их тезисы:

- dormant-first session creation
- history replay
- multi-view resize coordination
- IME/touch input bridge
- link/bell/title hooks

### Что в нём реально ценного

Из всех найденных проектов именно `floeterm` наиболее явно говорит:

"не хотим навязывать вам наш UI, хотим дать terminal plumbing для вашего продукта"

Это очень близко к нашей задаче.

### Самый важный technical caveat

⚠️ Web package пока зависит от prerelease Ghostty build:

- `@floegence/floeterm-terminal-web 0.4.16`
- dependency: `ghostty-web 0.4.0-next.14.g6a1a50d`

То есть даже их "готовый" web layer пока сидит на next-ветке underlying engine.

### Где риск

- почти нет adoption signal
- Go backend нужен не всем
- foundation выглядит интереснее, чем реально доказан

### Вывод

`floeterm` - очень интересный архитектурный донор и хороший candidate для дальнейшего наблюдения, но пока не лучший safe default.

### Оценка

`🎯 7   🛡️ 4   🧠 7`  
Примерно `3500-7000` строк.

---

## 5. `libghostty` напрямую

### Snapshot

- Project: [`ghostty-org/ghostty`](https://github.com/ghostty-org/ghostty)
- Related demo: [`ghostty-org/ghostling`](https://github.com/ghostty-org/ghostling)
- Ghostling stars: `952`

### Что подтверждают официальные источники

Официальные Ghostty sources и docs говорят:

- `libghostty-vt` уже usable today
- совместим с `macOS`, `Linux`, `Windows`, `WebAssembly`
- core functionality очень стабильна
- но API всё ещё **не заявлен как stable standalone library**

Это очень важная развилка.

### Почему это может быть самым сильным long-term path

Если смотреть на рынок на 1-2 года вперёд, то именно `libghostty` может стать новым центром тяжести для embeddable terminals.

Причины:

- сильная correctness история
- хорошая Unicode story
- rendering / parser / state model уже проверены боевым приложением
- вокруг него быстро растёт ecosystem

### Почему это пока не лучший immediate default

⚠️ Из-за API churn и большего объёма своей обвязки.

В нашем случае direct `libghostty` путь означает:

- больше своей glue-инфраструктуры
- больше рисков в сборке и portability
- больше своей ответственности за abstractions

### Вывод

`libghostty` надо держать как strategic direction, но не как безусловный "бери завтра в production" выбор.

### Оценка

`🎯 7   🛡️ 5   🧠 9`  
Примерно `6000-12000` строк.

---

## Что это значит для нашего репо

Если выбирать foundation под новый `src/features/terminal/`:

1. **`wterm`** - лучший balance embedability / modern UX
2. **`restty`** - лучший balance richness / terminal stack feel
3. **`ghostty-web`** - лучший lower-level base, если хотим больше контроля

`floeterm` полезно изучать как architectural donor, но не как safe default.

## Что foundation всё равно не решает

Ни один из сильных кандидатов не даёт из коробки всё, что нужно для IDE-grade terminal feature:

- command blocks
- shell integration
- recent commands / history palette
- notifications / attention routing
- worktree model
- session persistence
- global search / control center

Это нужно собирать отдельно.

## Sources

- [wterm](https://github.com/vercel-labs/wterm)
- [@wterm/react](https://www.npmjs.com/package/@wterm/react)
- [restty](https://github.com/wiedymi/restty)
- [restty npm](https://www.npmjs.com/package/restty)
- [ghostty-web](https://github.com/coder/ghostty-web)
- [ghostty-web npm](https://www.npmjs.com/package/ghostty-web)
- [floeterm](https://github.com/floegence/floeterm)
- [@floegence/floeterm-terminal-web](https://www.npmjs.com/package/@floegence/floeterm-terminal-web)
- [Ghostty About](https://ghostty.org/docs/about)
- [Ghostty repo](https://github.com/ghostty-org/ghostty)
- [Ghostling](https://github.com/ghostty-org/ghostling)
- [Libghostty Is Coming](https://mitchellh.com/writing/libghostty-is-coming)
