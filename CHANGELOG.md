# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## [v0.1.0](https://github.com/ailuracollective/colander/compare/ef64b7bd5b43d033338c57e0f2865014a1101e35..v0.1.0) - 2026-09-25
#### Features
- (**codec**) add an injectable wire codec seam - ([1ba24ed](https://github.com/ailuracollective/colander/commit/1ba24ed6ff8b82e6398f94ed63216f6892d41354)) - Pablo Santiago Guerra
- (**rules**) evaluate repeater calculations per row - ([b201ae1](https://github.com/ailuracollective/colander/commit/b201ae18e28a20f43f39b46e72ea5c62ec38cdda)) - Pablo Santiago Guerra
- add JSON Schema templates for forms, UI, rules, and golden vectors - ([58a3675](https://github.com/ailuracollective/colander/commit/58a36752460fe187c1b230e85da46e81ba087cc7)) - Pablo Santiago Guerra
#### Bug Fixes
- (**ci**) remove the cargo-make archive recursively in bump.yml - ([b4b7b3e](https://github.com/ailuracollective/colander/commit/b4b7b3e73aa0faab9ca64db290c265a0381b94c7)) - Pablo Santiago Guerra
- (**compile**) verify a component content hash - ([5b84acc](https://github.com/ailuracollective/colander/commit/5b84acc4f9e29f58d977679c641b9d98e9111cb9)) - Pablo Santiago Guerra
- (**ffi**) keep a panic in the final encode from unwinding - ([58138ba](https://github.com/ailuracollective/colander/commit/58138ba1e118c284e9493bc2f841e7d8c4ee7647)) - Pablo Santiago Guerra
- (**ffi**) reject a wrong-typed optional request key - ([7b2a293](https://github.com/ailuracollective/colander/commit/7b2a2937bcac6cf94c727e56830444e1ab9fd063)) - Pablo Santiago Guerra
- (**release**) extract bracketed changelog sections - ([6d0a361](https://github.com/ailuracollective/colander/commit/6d0a361dfedc15e12883297be299928a1b197047)) - Pablo Santiago Guerra
- (**release**) authenticate gh in the release step - ([bcd6f7f](https://github.com/ailuracollective/colander/commit/bcd6f7f5cf7f7a56da84797cc31ca4b3e0322966)) - Pablo Santiago Guerra
- (**release**) set the changelog remote to origin - ([0922252](https://github.com/ailuracollective/colander/commit/0922252fbecf71d0f80d52246ac1d783d5b6cec5)) - Pablo Santiago Guerra
- (**rules**) reject a validation entry without assert - ([f23b4cf](https://github.com/ailuracollective/colander/commit/f23b4cf48d1dfc413af8893975520b8f92b633f3)) - Pablo Santiago Guerra
- (**rules**) run the dependency check in every entry point - ([0892d55](https://github.com/ailuracollective/colander/commit/0892d553b6747a0542823569713c91057b66a588)) - Pablo Santiago Guerra
- (**schema**) bound evaluation work, nesting and error collection - ([4b5ba12](https://github.com/ailuracollective/colander/commit/4b5ba1276c83770d83b9c06f71a4d6568b431861)) - Pablo Santiago Guerra
- (**schema**) require schemas to be an object in every kind - ([3cab6e8](https://github.com/ailuracollective/colander/commit/3cab6e8a82ba00518569f0a92bb7abe8ee6f43aa)) - Pablo Santiago Guerra
- (**schema**) assert format for the closed set - ([5ffa883](https://github.com/ailuracollective/colander/commit/5ffa883a4d62015f1ec77ee6146e3af734cf1c34)) - Pablo Santiago Guerra
- (**schema**) compile patterns with fancy-regex and report truncation - ([2400481](https://github.com/ailuracollective/colander/commit/2400481015961ec9db0e0bd3f0a2e4a61fb91d0c)) - Pablo Santiago Guerra
- (**schema**) classify keywords and reject unsupported assertions - ([eb58d55](https://github.com/ailuracollective/colander/commit/eb58d556211858f010b5a574219ddf460364d784)) - Pablo Santiago Guerra
- (**schema**) retire published for workflows - ([5787b6c](https://github.com/ailuracollective/colander/commit/5787b6c39b81694d46695a3ab6ec477c5cba3e49)) - Pablo Santiago Guerra
- (**semver**) require strict triplets and an explicit bump - ([4b8ce59](https://github.com/ailuracollective/colander/commit/4b8ce59b54f04dbb41c58e2c74e964f21ae311ff)) - Pablo Santiago Guerra
- (**validate**) accept array answers and stop aborting on bad ones - ([0c6d51e](https://github.com/ailuracollective/colander/commit/0c6d51ea251b58ef184159542e2a263a7a5c6a12)) - Pablo Santiago Guerra
- close the confirmed findings of the security audit - ([80c8f35](https://github.com/ailuracollective/colander/commit/80c8f358f33983efe6aad46f753db28321934938)) - Pablo Santiago Guerra
- reject recursive $ref and compare integers exactly - ([2072822](https://github.com/ailuracollective/colander/commit/2072822b60bc9eb9949d44e5bb279218df257ce4)) - Pablo Santiago Guerra
- make analysis linear, bound error payloads, exact integers - ([e442c7a](https://github.com/ailuracollective/colander/commit/e442c7a885235945b8d0a250659f6a2ac2acda92)) - Pablo Santiago Guerra
- bound expansion, unify numeric equality, cap errors - ([d5d8173](https://github.com/ailuracollective/colander/commit/d5d8173931619000d11babaf99b82b2a0452a839)) - Pablo Santiago Guerra
- resolve adversarial audit findings (rules, validation, compile) - ([b675207](https://github.com/ailuracollective/colander/commit/b675207a044b7b288fdd02c77a675c8ddeb81fa6)) - Pablo Santiago Guerra
#### Documentation
- (**contract**) correct the C conformance row after C-8 - ([ad99ec3](https://github.com/ailuracollective/colander/commit/ad99ec36c99161ed8ddf5878aafff109b22fa5bc)) - Pablo Santiago Guerra
- (**contract**) mark C-8 live; it shipped with P-3 - ([4a1c559](https://github.com/ailuracollective/colander/commit/4a1c559aa223805d23635f2604eb0e24922e6f25)) - Pablo Santiago Guerra
- (**contract**) record four decisions and pin null as a wrong type - ([f1ba0ed](https://github.com/ailuracollective/colander/commit/f1ba0ed867f442d172e911bf2b8409cd5ba1cd0b)) - Pablo Santiago Guerra
- correct the WASM cost measurement and audit gotchas - ([4d335e6](https://github.com/ailuracollective/colander/commit/4d335e6a3107b227dff7fa800e9ce772d913a171)) - Pablo Santiago Guerra
- name both files over the 400-line limit - ([824f114](https://github.com/ailuracollective/colander/commit/824f1144570112e1dbb92cd5fd41a82aa610a296)) - Pablo Santiago Guerra
- use key-sorted form in place of canonical - ([3c59c3d](https://github.com/ailuracollective/colander/commit/3c59c3d830a5760895c8f9b791b017d5f13f4db6)) - Pablo Santiago Guerra
- add the contract of record - ([e01c0d8](https://github.com/ailuracollective/colander/commit/e01c0d8caa7940c82551fac6e61b42777a196249)) - Pablo Santiago Guerra
- allow decided contract changes to move the frozen vectors - ([293bc39](https://github.com/ailuracollective/colander/commit/293bc39af18034b2db4350063b0520aac220570e)) - Pablo Santiago Guerra
- correct stale contract and repository-state claims - ([fb0bb38](https://github.com/ailuracollective/colander/commit/fb0bb3800083a8808cc6b340184c0a801907eca2)) - Pablo Santiago Guerra
#### Tests
- add an engine benchmark for per-call cost - ([377d3ca](https://github.com/ailuracollective/colander/commit/377d3ca27625ead41ea6c221a9c3b6577078abdf)) - Pablo Santiago Guerra
- split schema format tests and tabulate request-key checks - ([7e658a7](https://github.com/ailuracollective/colander/commit/7e658a7072e8998e14ac0b1a1f8a699f08e4e841)) - Pablo Santiago Guerra
- split schema format tests and tabulate request-key checks - ([32e40b2](https://github.com/ailuracollective/colander/commit/32e40b24002616eef3eda14353dbf0abba6a6792)) - Pablo Santiago Guerra
- add a docs-truth guard and rename a misnamed test - ([e399400](https://github.com/ailuracollective/colander/commit/e399400bc3c8d177723779c1116e64293fc5f349)) - Pablo Santiago Guerra
#### Continuous Integration
- (**release**) automate the version bump and tag on master (#9) - ([bd05a75](https://github.com/ailuracollective/colander/commit/bd05a759c93a00078b98046b264bf720622eb037)) - Pablo Santiago Guerra
- replace a stale bump branch before pushing (#19) - ([27b591b](https://github.com/ailuracollective/colander/commit/27b591b05a0b7f759161527db21c86edf0a2f7b7)) - Pablo Santiago Guerra
- open the version PR on a push, not a pull_request event (#17) - ([ac52357](https://github.com/ailuracollective/colander/commit/ac52357bcd6b0f8454b18430f773b7bcfda7094e)) - Pablo Santiago Guerra
- give the bump guard a GH_TOKEN (#15) - ([d4bd5ae](https://github.com/ailuracollective/colander/commit/d4bd5aed26b0fbd206cf8e56b4bc429f4ae57f44)) - Pablo Santiago Guerra
- land the version bump as a pull request (#13) - ([bad641d](https://github.com/ailuracollective/colander/commit/bad641dc688c3c0e010baf3dfbac01d740f29dc1)) - Pablo Santiago Guerra
- pin the runner label and the checkout major (#11) - ([24593e3](https://github.com/ailuracollective/colander/commit/24593e375ec0321c9fa39d08995def4f630864d3)) - Pablo Santiago Guerra
- create the GitHub Release only on demand - ([6a057bf](https://github.com/ailuracollective/colander/commit/6a057bf0cbad6aa37dcac614459c8f20ab07b038)) - Pablo Santiago Guerra
#### Refactoring
- (**schema**) bundle the evaluator's shared state in a context - ([e552e6b](https://github.com/ailuracollective/colander/commit/e552e6bb49fd849f2c2e75e6b5e1630af16e8504)) - Pablo Santiago Guerra
- centralize semantic form projections (#5) - ([f3d577d](https://github.com/ailuracollective/colander/commit/f3d577d7ab93b13e2534a7ae0a7df090dcfd3001)) - Pablo Santiago Guerra
#### Miscellaneous Chores
- (**version**) v0.1.0 - ([64eb9a9](https://github.com/ailuracollective/colander/commit/64eb9a9d2c169328a6872eab90a011f6afb6d880)) - Pablo Santiago Guerra
- exclude generated changelog from dprint - ([742c208](https://github.com/ailuracollective/colander/commit/742c208dcb1d712b3e51aec7c663dcaa53062eaa)) - Pablo Santiago Guerra
- initialize colander - ([ef64b7b](https://github.com/ailuracollective/colander/commit/ef64b7bd5b43d033338c57e0f2865014a1101e35)) - Pablo Santiago Guerra

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).