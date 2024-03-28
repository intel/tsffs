; NOTES:
; Arguments:    RCX, RDX, R8, R9
; Callee-saved: RBX, RBP, RDI, RSI, RSP, R12, R13, R14, R15, XMM6-15
; Caller-saved: RAX, RCX, RDX, R8, R9, R10, R11, XMM0-5
; Return Value: RAX
;
; Test with: "W:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.31.31103\bin\Hostx64\x64\ml64.exe" /c /Cp /Cx /Zf tsffs-msvc-x86_64.asm

.CODE

HARNESS_START PROC
    push RDI
    push RSI
    push RBX

    mov RDI, 00h
    mov RSI, RCX
    ; mov RDX, RDX ; Unnecessary
    mov RAX, 014711h

    cpuid

    pop RBX
    pop RSI
    pop RDI

    ret
HARNESS_START ENDP

HARNESS_START_INDEX PROC
    push RDI
    push RSI
    push RBX

    mov RDI, RCX
    mov RSI, RDX
    mov RDX, R8
    mov RAX, 014711h

    cpuid

    pop RBX
    pop RSI
    pop RDI

    ret
HARNESS_START_INDEX ENDP

HARNESS_START_WITH_MAXIMUM_SIZE PROC
    push RDI
    push RSI
    push RBX

    mov RDI, 00h
    mov RSI, RCX
    ; mov RDX, RDX ; Unnecessary
    mov RAX, 024711h

    cpuid

    pop RBX
    pop RSI
    pop RDI

    ret
HARNESS_START_WITH_MAXIMUM_SIZE ENDP

HARNESS_START_WITH_MAXIMUM_SIZE_INDEX PROC
    push RDI
    push RSI
    push RBX

    mov RDI, RCX
    mov RSI, RDX
    mov RDX, R8
    mov RAX, 024711h

    cpuid

    pop RBX
    pop RSI
    pop RDI

    ret
HARNESS_START_WITH_MAXIMUM_SIZE_INDEX ENDP

HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR PROC
    push RDI
    push RSI
    push RBX

    mov RDI, 00h
    mov RSI, RCX
    ; mov RDX, RDX ; Unnecessary
    mov RCX, R8
    mov RAX, 034711h

    cpuid

    pop RBX
    pop RSI
    pop RDI

    ret
HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR ENDP

HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX PROC
    push RDI
    push RSI
    push RBX

    mov RDI, RCX
    mov RSI, RDX
    mov RDX, R8
    mov RCX, R9
    mov RAX, 034711h

    cpuid

    pop RBX
    pop RSI
    pop RDI

    ret
HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX ENDP

HARNESS_STOP PROC
    push RDI
    push RBX

    mov RDI, 00h
    mov RAX, 044711h

    cpuid

    pop RBX
    pop RDI

    ret
HARNESS_STOP ENDP

HARNESS_STOP_INDEX PROC
    push RDI
    push RBX

    mov RDI, RCX
    mov RAX, 044711h

    cpuid

    pop RBX
    pop RDI

    ret
HARNESS_STOP_INDEX ENDP

HARNESS_ASSERT PROC
    push RDI
    push RBX

    mov RDI, 00h
    mov RAX, 054711h

    cpuid

    pop RBX
    pop RDI

    ret
HARNESS_ASSERT ENDP

HARNESS_ASSERT_INDEX PROC
    push RDI
    push RBX

    mov RDI, RCX
    mov RAX, 054711h

    cpuid

    pop RBX
    pop RDI

    ret
HARNESS_ASSERT_INDEX ENDP

END