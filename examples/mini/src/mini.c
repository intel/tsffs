#include <stddef.h>
#include <stdint.h>

typedef struct EfiTableHeader {
  uint64_t signature;
  uint32_t revision;
  uint32_t headerSize;
  uint32_t crc32;
  uint32_t reserved;
} EfiTableHeader;

struct EfiSimpleTextOutputProtocol;

typedef uint64_t (*EfiTextString)(struct EfiSimpleTextOutputProtocol *this,
                                  int16_t *string);

typedef struct EfiSimpleTextOutputProtocol {
  uint64_t reset;
  EfiTextString output_string;
  uint64_t test_string;
  uint64_t query_mode;
  uint64_t set_mode;
  uint64_t set_attribute;
  uint64_t clear_screen;
  uint64_t set_cursor_position;
  uint64_t enable_cursor;
  uint64_t mode;
} EfiSimpleTextOutputProtocol;

typedef struct EfiSystemTable {
  EfiTableHeader hdr;
  int16_t *firmwareVendor;
  uint32_t firmwareRevision;
  void *consoleInHandle;
  uint64_t conIn;
  void *consoleOutHandle;
  EfiSimpleTextOutputProtocol *conOut;
  void *standardErrorHandle;
  uint64_t stdErr;
  uint64_t runtimeServices;
  uint64_t bootServices;
  uint64_t numberOfTableEntries;
  uint64_t configurationTable;
} EfiSystemTable;

const char hex[] = {'0', '1', '2', '3', '4', '5', '6', '7',
                    '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'};
const char *password = "f148{fuzz_m3}";

int Check(int16_t *buffer, EfiSystemTable *SystemTable) {
  SystemTable->conOut->output_string(SystemTable->conOut,
                                     (int16_t *)L"Checking Password!\r\n");
  if ((((char *)buffer)[0]) == password[0]) {
    if ((((char *)buffer)[1]) == password[1]) {
      if ((((char *)buffer)[2]) == password[2]) {
        if ((((char *)buffer)[3]) == password[3]) {
          if ((((char *)buffer)[4]) == password[4]) {
            if ((((char *)buffer)[5]) == password[5]) {
              if ((((char *)buffer)[6]) == password[6]) {
                if ((((char *)buffer)[7]) == password[7]) {
                  if ((((char *)buffer)[8]) == password[8]) {
                    if ((((char *)buffer)[9]) == password[9]) {
                      if ((((char *)buffer)[10]) == password[10]) {
                        if ((((char *)buffer)[11]) == password[11]) {
                          if ((((char *)buffer)[12]) == password[12]) {
                            // Crash!
                            SystemTable->conOut->output_string(
                                SystemTable->conOut,
                                (int16_t *)L"All characters were correct!\r\n");
                            uint8_t *ptr = (uint8_t *)0xffffffffffffffff;
                            *ptr = 0;
                          } else {
                            SystemTable->conOut->output_string(
                                SystemTable->conOut,
                                (int16_t *)L"Char 12 was wrong!\r\n");
                          }
                        } else {
                          SystemTable->conOut->output_string(
                              SystemTable->conOut,
                              (int16_t *)L"Char 11 was wrong!\r\n");
                        }
                      } else {
                        SystemTable->conOut->output_string(
                            SystemTable->conOut,
                            (int16_t *)L"Char 10 was wrong!\r\n");
                      }
                    } else {
                      SystemTable->conOut->output_string(
                          SystemTable->conOut,
                          (int16_t *)L"Char 9 was wrong!\r\n");
                    }
                  } else {
                    SystemTable->conOut->output_string(
                        SystemTable->conOut,
                        (int16_t *)L"Char 8 was wrong!\r\n");
                  }
                } else {
                  SystemTable->conOut->output_string(
                      SystemTable->conOut, (int16_t *)L"Char 7 was wrong!\r\n");
                }
              } else {
                SystemTable->conOut->output_string(
                    SystemTable->conOut, (int16_t *)L"Char 6 was wrong!\r\n");
              }
            } else {
              SystemTable->conOut->output_string(
                  SystemTable->conOut, (int16_t *)L"Char 5 was wrong!\r\n");
            }
          } else {
            SystemTable->conOut->output_string(
                SystemTable->conOut, (int16_t *)L"Char 4 was wrong!\r\n");
          }
        } else {
          SystemTable->conOut->output_string(
              SystemTable->conOut, (int16_t *)L"Char 3 was wrong!\r\n");
        }
      } else {
        SystemTable->conOut->output_string(SystemTable->conOut,
                                           (int16_t *)L"Char 2 was wrong!\r\n");
      }
    } else {
      SystemTable->conOut->output_string(SystemTable->conOut,
                                         (int16_t *)L"Char 1 was wrong!\r\n");
    }
  } else {
    SystemTable->conOut->output_string(SystemTable->conOut,
                                       (int16_t *)L"Char 0 was wrong!\r\n");
  }
  return 0;
}

// The entrypoint of our EFI application
int UefiMain(void *imageHandle, EfiSystemTable *SystemTable) {
  // We will store the CPUID results we obtain here, they won't be used.
  uint32_t _a, _b, _c, _d = 0;

  // We have a size and a buffer of that size. The address of the buffer and the
  // address of the size variable will be passed to the fuzzer. On the first
  // start harness, the fuzzer will save the initial value of the size and the
  // addresses of both variables. On each iteration of the fuzzer, up to the
  // initial size bytes of fuzzer input data will be written to the buffer, and
  // the current testcase size in bytes will be written to the size variable.
  int16_t buffer[0x20];
  size_t size = sizeof(buffer) - 1;
  int16_t *buffer_ptr = &buffer[0];

  // Our "start harness" is just a CPUID with some special values:
  // - (0x43434711 indicates START, which is just our START signal 0x4343
  //   shifted left 16 bits, ORed with the signal for SIMICS that this is a
  //   magic instruction, 0x4711).
  // - The buffer address
  __asm__ __volatile__(
      "cpuid\n\t"
      : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d), "=S"(buffer_ptr), "=D"(size)
      : "0"((0x4343U << 16U) | 0x4711U), "S"(buffer_ptr), "D"(size));

  // Once we reach this point, the fuzzer has filled our buffer with some amount
  // of data. We will print out the data we got.

  for (size_t i = 0; i < size; i++) {
    if (i != 0 && !(i % 8)) {
      SystemTable->conOut->output_string(SystemTable->conOut,
                                         (int16_t *)L"\r\n");
    }
    int16_t buf[5];
    buf[4] = 0;
    int16_t chr = buffer[i];
    buf[0] = hex[chr & 0xf];
    buf[1] = hex[(chr >> 4) & 0xf];
    buf[2] = hex[(chr >> 8) & 0xf];
    buf[3] = hex[(chr >> 12) & 0xf];

    SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)&buf[0]);
  }

  SystemTable->conOut->output_string(SystemTable->conOut, (int16_t *)L"\r\n");

  Check(buffer, SystemTable);

  // We've run the code we wanted to test, so we now trigger our "stop harness",
  // which is another CPUID with another magic value, this time signaling the
  // fuzzer that this is the end of the harness and we should reset to the
  // beginning.

  __asm__ __volatile__("cpuid\n\t"
                       : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d)
                       : "0"((0x4242U << 16U) | 0x4711U));

  return 0;
}