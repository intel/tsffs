// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

// Synthetic stand-in for an EDK2 DXE-library function.
//
// This is NOT real EDK2/UEFI source. It exists purely to give gcc something
// to compile with `-g` so the resulting ELF's DWARF debug info can be used
// as an offline test fixture for `DwarfModule::intervals()` (see
// ../../../src/dwarf/mod.rs). The name/shape (`X509VerifyCert` in a file
// named `X509CertVerify.c`) mirrors the real, public EDK2 `BaseCryptLib`
// certificate-verification API this project's own edk2-uefi tutorial
// harnesses (see docs/src/tutorials/edk2-uefi/writing-the-application.md),
// giving the fixture a realistic, non-zero link-time `.text` VMA without
// requiring a full EDK2/Docker BIOS build.
//
// Regenerate the compiled fixture with:
//
//   gcc -g -O0 -ffreestanding -fno-stack-protector -fno-builtin \
//       -c X509CertVerify.c -o X509CertVerify.o
//   ld -o X509CertVerify.debug X509CertVerify.o --entry=X509VerifyCert \
//       --section-start=.text=0x240
//
// Ground truth for the test (recorded here so it stays traceable if the
// toolchain changes and someone regenerates the fixture):
//   - `.text` link-time VMA: 0x240 (asked for explicitly via
//     `--section-start`; confirmed actual with `objdump -h`)
//   - `X509VerifyCert` link-time address/size: read from `nm`/`objdump`
//     against the checked-in `X509CertVerify.debug`, see the doc-comment on
//     the test in `../../../src/dwarf/mod.rs`.

typedef unsigned long UINTN;
typedef unsigned char UINT8;

// Sums the bytes of a certificate. Standing in for whatever real ASN.1/DER
// parsing a certificate-verification routine would do to caller-supplied
// certificate data before checking it against a trust anchor.
static UINTN HashCertificateBytes(const UINT8 *Cert, UINTN CertSize) {
  UINTN Checksum = 0;

  for (UINTN Index = 0; Index < CertSize; Index++) {
    Checksum += Cert[Index];
  }

  return Checksum;
}

// Synthetic analog of a real EDK2 certificate-verification routine, e.g.
// BaseCryptLib's X509VerifyCert.
UINTN X509VerifyCert(const UINT8 *Cert, UINTN CertSize, UINT8 *Data) {
  UINTN Checksum = HashCertificateBytes(Cert, CertSize);

  if (Checksum == 0) {
    return 0;
  }

  Data[0] = (UINT8)(Checksum & 0xFF);

  return Checksum;
}
