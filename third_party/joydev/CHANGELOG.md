## 0.3.1

- Make it so the project only builds on Linux (joydev isn't available elsewhere anyways)

## 0.3.0

- Rename `KeyOrButton` to `Key`
- Add ability to iterate over `AbsoluteAxis` and `Key`
- Move count and max to `EventCode` trait
- Update `joydev-sys` and add some missing traits

## 0.2.0

- Add documentation
- Rename some constants in `AbsoluteAxis`
- Add missing constants in `KeyOrButton`
- Rename some constants in `KeyOrButton`

## 0.1.1

- Add missing methods
    - `Device::identifier`
    - `Correction::coefficient_mut`
    - `Correction::set_precision`
    - `Correction::set_type`
- Add some documentation

## 0.1.0

- Initial release.