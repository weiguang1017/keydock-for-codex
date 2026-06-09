#define CKM_TESTING 1
#import "../src/main.m"

static void CKMAssert(BOOL condition, NSString *message) {
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", message.UTF8String);
        exit(1);
    }
}

static NSString *CKMTestTempDirectory(NSString *name) {
    NSString *path = [NSTemporaryDirectory() stringByAppendingPathComponent:[NSString stringWithFormat:@"ckm-%@-%@", name, NSUUID.UUID.UUIDString]];
    NSError *error = nil;
    BOOL ok = [NSFileManager.defaultManager createDirectoryAtPath:path withIntermediateDirectories:YES attributes:nil error:&error];
    CKMAssert(ok, [NSString stringWithFormat:@"create temp dir: %@", error.localizedDescription]);
    return path;
}

static void testMasking(void) {
    CKMAssert([CKMMaskKey(@"sk-1234567890abcdef") isEqualToString:@"sk-1234...cdef"], @"mask long key");
    CKMAssert([CKMMaskKey(@"abcd1234") isEqualToString:@"***"], @"mask short key");
    CKMAssert([CKMMaskKey(@"  sk-short123  ") isEqualToString:@"sk-...123"], @"trim and mask medium key");
}

static void testMetadataStore(void) {
    NSString *directory = CKMTestTempDirectory(@"metadata");
    NSURL *url = [[NSURL fileURLWithPath:directory isDirectory:YES] URLByAppendingPathComponent:@"keys.json"];
    CKMMetadataStore *store = [[CKMMetadataStore alloc] initWithFileURL:url];
    CKMKeyRecord *record = [CKMKeyRecord recordWithLabel:@"Work" apiKey:@"sk-test-1234567890"];
    record.active = YES;

    NSError *error = nil;
    CKMAssert([store saveRecords:@[record] error:&error], [NSString stringWithFormat:@"save metadata: %@", error.localizedDescription]);
    NSMutableArray<CKMKeyRecord *> *records = [store loadRecordsWithError:&error];
    CKMAssert(records.count == 1, @"load one record");
    CKMAssert([records.firstObject.label isEqualToString:@"Work"], @"load label");
    CKMAssert(records.firstObject.active, @"load active");
    NSString *json = [NSString stringWithContentsOfURL:url encoding:NSUTF8StringEncoding error:&error];
    CKMAssert(![json containsString:@"sk-test-1234567890"], @"metadata does not contain secret");
}

static void testKeychain(void) {
    NSString *directory = CKMTestTempDirectory(@"keychain");
    setenv("CKM_TEST_KEYCHAIN_DIR", directory.UTF8String, 1);
    NSString *service = [NSString stringWithFormat:@"KeydockForCodexTests-%@", NSUUID.UUID.UUIDString];
    setenv("CKM_KEYCHAIN_SERVICE", service.UTF8String, 1);

    NSString *account = NSUUID.UUID.UUIDString;
    NSString *secret = @"sk-test-keychain-1234567890";
    NSError *error = nil;
    CKMAssert([CKMKeychain saveSecret:secret account:account error:&error], [NSString stringWithFormat:@"save keychain: %@", error.localizedDescription]);
    NSString *loaded = [CKMKeychain readSecretForAccount:account error:&error];
    CKMAssert([loaded isEqualToString:secret], @"read keychain secret");
    CKMAssert([CKMKeychain deleteSecretForAccount:account error:&error], [NSString stringWithFormat:@"delete keychain: %@", error.localizedDescription]);
    NSString *missing = [CKMKeychain readSecretForAccount:account error:nil];
    CKMAssert(missing == nil, @"deleted keychain secret is missing");
}

static void writeFakeCodex(NSString *path, NSString *capturePath) {
    NSString *script = [NSString stringWithFormat:
        @"#!/bin/sh\n"
         "if [ \"$1\" = \"login\" ] && [ \"$2\" = \"--with-api-key\" ]; then\n"
         "  IFS= read -r KEY\n"
         "  printf '%%s' \"$KEY\" > '%@'\n"
         "  printf 'login ok\\n'\n"
         "  exit 0\n"
         "fi\n"
         "if [ \"$1\" = \"login\" ] && [ \"$2\" = \"status\" ]; then\n"
         "  printf 'Logged in using an API key - sk-test***7890\\n'\n"
         "  exit 0\n"
         "fi\n"
         "printf 'unexpected args: %%s %%s\\n' \"$1\" \"$2\" >&2\n"
         "exit 2\n", capturePath];
    NSError *error = nil;
    BOOL ok = [script writeToFile:path atomically:YES encoding:NSUTF8StringEncoding error:&error];
    CKMAssert(ok, [NSString stringWithFormat:@"write fake codex: %@", error.localizedDescription]);
    ok = [NSFileManager.defaultManager setAttributes:@{NSFilePosixPermissions: @0755} ofItemAtPath:path error:&error];
    CKMAssert(ok, [NSString stringWithFormat:@"chmod fake codex: %@", error.localizedDescription]);
}

static void testCodexCliDetectionAndStdin(void) {
    NSString *directory = CKMTestTempDirectory(@"codex");
    NSString *fakeCodex = [directory stringByAppendingPathComponent:@"codex"];
    NSString *capture = [directory stringByAppendingPathComponent:@"capture.txt"];
    writeFakeCodex(fakeCodex, capture);

    setenv("CKM_CODEX_PATH", fakeCodex.UTF8String, 1);
    NSString *oldPath = NSProcessInfo.processInfo.environment[@"PATH"] ?: @"";
    NSString *newPath = [NSString stringWithFormat:@"%@:%@", directory, oldPath];
    setenv("PATH", newPath.UTF8String, 1);

    NSError *error = nil;
    NSString *codexPath = [CKMCodexService findCodexPath:&error];
    CKMAssert([codexPath isEqualToString:fakeCodex], [NSString stringWithFormat:@"find fake codex: %@", error.localizedDescription]);

    NSString *status = nil;
    NSString *apiKey = @"sk-test-stdin-1234567890";
    CKMAssert([CKMCodexService loginWithAPIKey:apiKey codexPath:codexPath statusOutput:&status error:&error],
              [NSString stringWithFormat:@"login fake codex: %@", error.localizedDescription]);
    NSString *captured = [NSString stringWithContentsOfFile:capture encoding:NSUTF8StringEncoding error:&error];
    CKMAssert([captured isEqualToString:apiKey], @"fake codex received stdin key");
    CKMAssert([status containsString:@"Logged in using an API key"], @"status output returned");
}

int main(int argc, const char *argv[]) {
    (void)argc;
    (void)argv;
    @autoreleasepool {
        setenv("CKM_DISABLE_RESTART", "1", 1);
        setenv("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS", "1", 1);
        testMasking();
        testMetadataStore();
        testKeychain();
        testCodexCliDetectionAndStdin();
        printf("PASS: Keydock for Codex native tests completed.\n");
    }
    return 0;
}
