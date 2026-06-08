#import <Cocoa/Cocoa.h>
#import <Security/Security.h>

static NSString * const CKMKeychainServiceDefault = @"KeydockForCodex";
static NSString * const CKMCodexBundleIdentifier = @"com.openai.codex";

static NSString *CKMTrim(NSString *value) {
    if (!value) {
        return @"";
    }
    return [value stringByTrimmingCharactersInSet:[NSCharacterSet whitespaceAndNewlineCharacterSet]];
}

NSString *CKMMaskKey(NSString *key) {
    NSString *trimmed = CKMTrim(key);
    NSUInteger length = trimmed.length;
    if (length == 0) {
        return @"";
    }
    if (length <= 8) {
        return @"***";
    }
    if (length <= 14) {
        return [NSString stringWithFormat:@"%@...%@", [trimmed substringToIndex:3], [trimmed substringFromIndex:length - 3]];
    }
    return [NSString stringWithFormat:@"%@...%@", [trimmed substringToIndex:7], [trimmed substringFromIndex:length - 4]];
}

static NSString *CKMNowString(void) {
    static NSISO8601DateFormatter *formatter;
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        formatter = [[NSISO8601DateFormatter alloc] init];
        formatter.formatOptions = NSISO8601DateFormatWithInternetDateTime;
    });
    return [formatter stringFromDate:[NSDate date]];
}

static NSString *CKMDisplayDate(NSString *isoString) __attribute__((unused));
static NSString *CKMDisplayDate(NSString *isoString) {
    if (isoString.length == 0) {
        return @"Never";
    }
    static NSISO8601DateFormatter *inputFormatter;
    static NSDateFormatter *outputFormatter;
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        inputFormatter = [[NSISO8601DateFormatter alloc] init];
        inputFormatter.formatOptions = NSISO8601DateFormatWithInternetDateTime;
        outputFormatter = [[NSDateFormatter alloc] init];
        outputFormatter.dateStyle = NSDateFormatterMediumStyle;
        outputFormatter.timeStyle = NSDateFormatterShortStyle;
    });
    NSDate *date = [inputFormatter dateFromString:isoString];
    if (!date) {
        return isoString;
    }
    return [outputFormatter stringFromDate:date];
}

static BOOL CKMSetError(NSError **error, NSString *domain, NSInteger code, NSString *message) {
    if (error) {
        *error = [NSError errorWithDomain:domain
                                     code:code
                                 userInfo:@{NSLocalizedDescriptionKey: message ?: @"Unknown error"}];
    }
    return NO;
}

static NSString *CKMKeychainService(void) {
    const char *service = getenv("CKM_KEYCHAIN_SERVICE");
    if (service && strlen(service) > 0) {
        return [NSString stringWithUTF8String:service];
    }
    return CKMKeychainServiceDefault;
}

static NSString *CKMTestKeychainDirectory(void) {
    const char *directory = getenv("CKM_TEST_KEYCHAIN_DIR");
    if (directory && strlen(directory) > 0) {
        return [NSString stringWithUTF8String:directory];
    }
    return nil;
}

static NSString *CKMSafeFilename(NSString *value) {
    NSMutableString *safe = [NSMutableString string];
    NSCharacterSet *allowed = [NSCharacterSet characterSetWithCharactersInString:@"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_."];
    for (NSUInteger index = 0; index < value.length; index++) {
        unichar character = [value characterAtIndex:index];
        if ([allowed characterIsMember:character]) {
            [safe appendFormat:@"%C", character];
        } else {
            [safe appendString:@"_"];
        }
    }
    return safe.length > 0 ? safe : @"empty";
}

static NSURL *CKMApplicationSupportDirectory(void) {
    const char *overridePath = getenv("CKM_STORAGE_DIR");
    if (overridePath && strlen(overridePath) > 0) {
        return [NSURL fileURLWithPath:[NSString stringWithUTF8String:overridePath] isDirectory:YES];
    }

    NSArray<NSString *> *paths = NSSearchPathForDirectoriesInDomains(NSApplicationSupportDirectory, NSUserDomainMask, YES);
    NSString *base = paths.firstObject ?: NSTemporaryDirectory();
    return [[NSURL fileURLWithPath:base isDirectory:YES] URLByAppendingPathComponent:@"Keydock for Codex" isDirectory:YES];
}

@interface CKMKeyRecord : NSObject
@property (nonatomic, copy) NSString *identifier;
@property (nonatomic, copy) NSString *label;
@property (nonatomic, copy) NSString *maskedKey;
@property (nonatomic, assign) BOOL active;
@property (nonatomic, copy) NSString *lastValidatedAt;
@property (nonatomic, copy) NSString *createdAt;
@property (nonatomic, copy) NSString *updatedAt;
+ (instancetype)recordWithLabel:(NSString *)label apiKey:(NSString *)apiKey;
+ (instancetype)recordFromDictionary:(NSDictionary *)dictionary;
- (NSDictionary *)dictionaryValue;
@end

@implementation CKMKeyRecord

+ (instancetype)recordWithLabel:(NSString *)label apiKey:(NSString *)apiKey {
    CKMKeyRecord *record = [[CKMKeyRecord alloc] init];
    record.identifier = [NSUUID UUID].UUIDString;
    record.label = CKMTrim(label).length > 0 ? CKMTrim(label) : @"Untitled key";
    record.maskedKey = CKMMaskKey(apiKey);
    record.active = NO;
    record.lastValidatedAt = CKMNowString();
    record.createdAt = CKMNowString();
    record.updatedAt = record.createdAt;
    return record;
}

+ (instancetype)recordFromDictionary:(NSDictionary *)dictionary {
    if (![dictionary isKindOfClass:NSDictionary.class]) {
        return nil;
    }
    CKMKeyRecord *record = [[CKMKeyRecord alloc] init];
    record.identifier = [dictionary[@"id"] isKindOfClass:NSString.class] ? dictionary[@"id"] : [NSUUID UUID].UUIDString;
    record.label = [dictionary[@"label"] isKindOfClass:NSString.class] ? dictionary[@"label"] : @"Untitled key";
    record.maskedKey = [dictionary[@"maskedKey"] isKindOfClass:NSString.class] ? dictionary[@"maskedKey"] : @"";
    record.active = [dictionary[@"active"] boolValue];
    record.lastValidatedAt = [dictionary[@"lastValidatedAt"] isKindOfClass:NSString.class] ? dictionary[@"lastValidatedAt"] : @"";
    record.createdAt = [dictionary[@"createdAt"] isKindOfClass:NSString.class] ? dictionary[@"createdAt"] : CKMNowString();
    record.updatedAt = [dictionary[@"updatedAt"] isKindOfClass:NSString.class] ? dictionary[@"updatedAt"] : record.createdAt;
    return record;
}

- (NSDictionary *)dictionaryValue {
    return @{
        @"id": self.identifier ?: @"",
        @"label": self.label ?: @"Untitled key",
        @"maskedKey": self.maskedKey ?: @"",
        @"active": @(self.active),
        @"lastValidatedAt": self.lastValidatedAt ?: @"",
        @"createdAt": self.createdAt ?: @"",
        @"updatedAt": self.updatedAt ?: @""
    };
}

@end

@interface CKMMetadataStore : NSObject
@property (nonatomic, strong, readonly) NSURL *fileURL;
- (instancetype)init;
- (instancetype)initWithFileURL:(NSURL *)fileURL;
- (NSMutableArray<CKMKeyRecord *> *)loadRecordsWithError:(NSError **)error;
- (BOOL)saveRecords:(NSArray<CKMKeyRecord *> *)records error:(NSError **)error;
@end

@implementation CKMMetadataStore

- (instancetype)init {
    NSURL *directory = CKMApplicationSupportDirectory();
    return [self initWithFileURL:[directory URLByAppendingPathComponent:@"keys.json"]];
}

- (instancetype)initWithFileURL:(NSURL *)fileURL {
    self = [super init];
    if (self) {
        _fileURL = fileURL;
    }
    return self;
}

- (NSMutableArray<CKMKeyRecord *> *)loadRecordsWithError:(NSError **)error {
    NSFileManager *fileManager = NSFileManager.defaultManager;
    if (![fileManager fileExistsAtPath:self.fileURL.path]) {
        return [NSMutableArray array];
    }

    NSData *data = [NSData dataWithContentsOfURL:self.fileURL options:0 error:error];
    if (!data) {
        return nil;
    }

    id json = [NSJSONSerialization JSONObjectWithData:data options:0 error:error];
    if (!json) {
        return nil;
    }

    NSArray *items = nil;
    if ([json isKindOfClass:NSDictionary.class]) {
        items = ((NSDictionary *)json)[@"keys"];
    } else if ([json isKindOfClass:NSArray.class]) {
        items = json;
    }
    if (![items isKindOfClass:NSArray.class]) {
        return [NSMutableArray array];
    }

    NSMutableArray<CKMKeyRecord *> *records = [NSMutableArray array];
    for (NSDictionary *item in items) {
        CKMKeyRecord *record = [CKMKeyRecord recordFromDictionary:item];
        if (record) {
            [records addObject:record];
        }
    }
    return records;
}

- (BOOL)saveRecords:(NSArray<CKMKeyRecord *> *)records error:(NSError **)error {
    NSFileManager *fileManager = NSFileManager.defaultManager;
    NSURL *directory = [self.fileURL URLByDeletingLastPathComponent];
    if (![fileManager fileExistsAtPath:directory.path]) {
        if (![fileManager createDirectoryAtURL:directory withIntermediateDirectories:YES attributes:nil error:error]) {
            return NO;
        }
    }

    NSMutableArray *items = [NSMutableArray arrayWithCapacity:records.count];
    for (CKMKeyRecord *record in records) {
        [items addObject:[record dictionaryValue]];
    }
    NSDictionary *root = @{@"keys": items};
    NSData *data = [NSJSONSerialization dataWithJSONObject:root options:NSJSONWritingPrettyPrinted error:error];
    if (!data) {
        return NO;
    }
    return [data writeToURL:self.fileURL options:NSDataWritingAtomic error:error];
}

@end

@interface CKMKeychain : NSObject
+ (BOOL)saveSecret:(NSString *)secret account:(NSString *)account error:(NSError **)error;
+ (NSString *)readSecretForAccount:(NSString *)account error:(NSError **)error;
+ (BOOL)deleteSecretForAccount:(NSString *)account error:(NSError **)error;
@end

@implementation CKMKeychain

+ (NSString *)testSecretPathForAccount:(NSString *)account {
    NSString *directory = CKMTestKeychainDirectory();
    if (directory.length == 0) {
        return nil;
    }
    return [directory stringByAppendingPathComponent:CKMSafeFilename(account ?: @"")];
}

+ (NSMutableDictionary *)baseQueryForAccount:(NSString *)account {
    return [@{
        (__bridge id)kSecClass: (__bridge id)kSecClassGenericPassword,
        (__bridge id)kSecAttrService: CKMKeychainService(),
        (__bridge id)kSecAttrAccount: account ?: @""
    } mutableCopy];
}

+ (NSString *)messageForStatus:(OSStatus)status fallback:(NSString *)fallback {
    CFStringRef message = SecCopyErrorMessageString(status, NULL);
    if (message) {
        return CFBridgingRelease(message);
    }
    return fallback;
}

+ (BOOL)saveSecret:(NSString *)secret account:(NSString *)account error:(NSError **)error {
    if (account.length == 0 || secret.length == 0) {
        return CKMSetError(error, @"KeydockForCodex.Keychain", -1, @"Missing keychain account or secret.");
    }
    NSString *testPath = [self testSecretPathForAccount:account];
    if (testPath) {
        NSString *directory = [testPath stringByDeletingLastPathComponent];
        if (![NSFileManager.defaultManager createDirectoryAtPath:directory withIntermediateDirectories:YES attributes:nil error:error]) {
            return NO;
        }
        BOOL ok = [secret writeToFile:testPath atomically:YES encoding:NSUTF8StringEncoding error:error];
        if (ok) {
            [NSFileManager.defaultManager setAttributes:@{NSFilePosixPermissions: @0600} ofItemAtPath:testPath error:nil];
        }
        return ok;
    }

    NSData *secretData = [secret dataUsingEncoding:NSUTF8StringEncoding];
    NSMutableDictionary *query = [self baseQueryForAccount:account];
    SecItemDelete((__bridge CFDictionaryRef)query);
    query[(__bridge id)kSecValueData] = secretData;
    OSStatus status = SecItemAdd((__bridge CFDictionaryRef)query, NULL);
    if (status != errSecSuccess) {
        NSString *message = [self messageForStatus:status fallback:@"Unable to save secret in Keychain."];
        return CKMSetError(error, @"KeydockForCodex.Keychain", status, message);
    }
    return YES;
}

+ (NSString *)readSecretForAccount:(NSString *)account error:(NSError **)error {
    if (account.length == 0) {
        CKMSetError(error, @"KeydockForCodex.Keychain", -1, @"Missing keychain account.");
        return nil;
    }
    NSString *testPath = [self testSecretPathForAccount:account];
    if (testPath) {
        if (![NSFileManager.defaultManager fileExistsAtPath:testPath]) {
            CKMSetError(error, @"KeydockForCodex.Keychain", errSecItemNotFound, @"Secret was not found.");
            return nil;
        }
        return [NSString stringWithContentsOfFile:testPath encoding:NSUTF8StringEncoding error:error];
    }

    NSMutableDictionary *query = [self baseQueryForAccount:account];
    query[(__bridge id)kSecReturnData] = @YES;
    query[(__bridge id)kSecMatchLimit] = (__bridge id)kSecMatchLimitOne;

    CFTypeRef result = NULL;
    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &result);
    if (status != errSecSuccess) {
        NSString *message = [self messageForStatus:status fallback:@"Unable to read secret from Keychain."];
        CKMSetError(error, @"KeydockForCodex.Keychain", status, message);
        return nil;
    }

    NSData *data = CFBridgingRelease(result);
    return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
}

+ (BOOL)deleteSecretForAccount:(NSString *)account error:(NSError **)error {
    if (account.length == 0) {
        return YES;
    }
    NSString *testPath = [self testSecretPathForAccount:account];
    if (testPath) {
        if (![NSFileManager.defaultManager fileExistsAtPath:testPath]) {
            return YES;
        }
        return [NSFileManager.defaultManager removeItemAtPath:testPath error:error];
    }

    NSMutableDictionary *query = [self baseQueryForAccount:account];
    OSStatus status = SecItemDelete((__bridge CFDictionaryRef)query);
    if (status != errSecSuccess && status != errSecItemNotFound) {
        NSString *message = [self messageForStatus:status fallback:@"Unable to delete secret from Keychain."];
        return CKMSetError(error, @"KeydockForCodex.Keychain", status, message);
    }
    return YES;
}

@end

@interface CKMValidationResult : NSObject
@property (nonatomic, assign) BOOL valid;
@property (nonatomic, assign) NSInteger statusCode;
@property (nonatomic, copy) NSString *message;
+ (instancetype)validResultWithMessage:(NSString *)message;
+ (instancetype)invalidResultWithStatusCode:(NSInteger)statusCode message:(NSString *)message;
@end

@implementation CKMValidationResult

+ (instancetype)validResultWithMessage:(NSString *)message {
    CKMValidationResult *result = [[CKMValidationResult alloc] init];
    result.valid = YES;
    result.statusCode = 200;
    result.message = message ?: @"Key is valid.";
    return result;
}

+ (instancetype)invalidResultWithStatusCode:(NSInteger)statusCode message:(NSString *)message {
    CKMValidationResult *result = [[CKMValidationResult alloc] init];
    result.valid = NO;
    result.statusCode = statusCode;
    result.message = message ?: @"Key check failed.";
    return result;
}

@end

@interface CKMOpenAIValidator : NSObject
- (void)validateKey:(NSString *)apiKey completion:(void (^)(CKMValidationResult *result))completion;
@end

@implementation CKMOpenAIValidator

- (NSURL *)validationURL {
    const char *overrideURL = getenv("CKM_VALIDATION_URL");
    NSString *urlString = (overrideURL && strlen(overrideURL) > 0) ? [NSString stringWithUTF8String:overrideURL] : @"https://api.openai.com/v1/models";
    return [NSURL URLWithString:urlString];
}

- (void)validateKey:(NSString *)apiKey completion:(void (^)(CKMValidationResult *result))completion {
    NSString *trimmed = CKMTrim(apiKey);
    if (![trimmed hasPrefix:@"sk-"]) {
        completion([CKMValidationResult invalidResultWithStatusCode:0 message:@"The key must start with sk-."]);
        return;
    }

    const char *skipValidation = getenv("CKM_SKIP_NETWORK_VALIDATION_FOR_TESTS");
    if (skipValidation && strcmp(skipValidation, "1") == 0) {
        completion([CKMValidationResult validResultWithMessage:@"Test validation passed."]);
        return;
    }

    NSURL *url = [self validationURL];
    if (!url) {
        completion([CKMValidationResult invalidResultWithStatusCode:0 message:@"Validation URL is invalid."]);
        return;
    }

    NSMutableURLRequest *request = [NSMutableURLRequest requestWithURL:url];
    request.HTTPMethod = @"GET";
    request.timeoutInterval = 20.0;
    [request setValue:[NSString stringWithFormat:@"Bearer %@", trimmed] forHTTPHeaderField:@"Authorization"];
    [request setValue:@"application/json" forHTTPHeaderField:@"Accept"];

    NSURLSessionDataTask *task = [NSURLSession.sharedSession dataTaskWithRequest:request
                                                               completionHandler:^(NSData *data, NSURLResponse *response, NSError *error) {
        (void)data;
        if (error) {
            completion([CKMValidationResult invalidResultWithStatusCode:0 message:error.localizedDescription]);
            return;
        }

        NSInteger statusCode = 0;
        if ([response isKindOfClass:NSHTTPURLResponse.class]) {
            statusCode = ((NSHTTPURLResponse *)response).statusCode;
        }

        if (statusCode == 200) {
            completion([CKMValidationResult validResultWithMessage:@"OpenAI accepted this key."]);
        } else if (statusCode == 401) {
            completion([CKMValidationResult invalidResultWithStatusCode:statusCode message:@"OpenAI rejected this key."]);
        } else if (statusCode == 403) {
            completion([CKMValidationResult invalidResultWithStatusCode:statusCode message:@"This key is not permitted to access the validation endpoint."]);
        } else {
            completion([CKMValidationResult invalidResultWithStatusCode:statusCode
                                                               message:[NSString stringWithFormat:@"Validation failed with HTTP %ld.", (long)statusCode]]);
        }
    }];
    [task resume];
}

@end

@interface CKMCommandResult : NSObject
@property (nonatomic, assign) int status;
@property (nonatomic, copy) NSString *standardOutput;
@property (nonatomic, copy) NSString *standardError;
@end

@implementation CKMCommandResult
@end

@interface CKMCommandRunner : NSObject
+ (CKMCommandResult *)runExecutable:(NSString *)path
                           arguments:(NSArray<NSString *> *)arguments
                         stdinString:(NSString *)stdinString
                             timeout:(NSTimeInterval)timeout
                               error:(NSError **)error;
@end

@implementation CKMCommandRunner

+ (CKMCommandResult *)runExecutable:(NSString *)path
                           arguments:(NSArray<NSString *> *)arguments
                         stdinString:(NSString *)stdinString
                             timeout:(NSTimeInterval)timeout
                               error:(NSError **)error {
    if (path.length == 0) {
        CKMSetError(error, @"KeydockForCodex.Command", -1, @"Missing executable path.");
        return nil;
    }

    NSTask *task = [[NSTask alloc] init];
    task.launchPath = path;
    task.arguments = arguments ?: @[];
    task.environment = NSProcessInfo.processInfo.environment;

    NSPipe *stdoutPipe = [NSPipe pipe];
    NSPipe *stderrPipe = [NSPipe pipe];
    task.standardOutput = stdoutPipe;
    task.standardError = stderrPipe;

    NSPipe *stdinPipe = nil;
    if (stdinString) {
        stdinPipe = [NSPipe pipe];
        task.standardInput = stdinPipe;
    }

    NSError *launchError = nil;
    if (![task launchAndReturnError:&launchError]) {
        if (error) {
            *error = launchError;
        }
        return nil;
    }

    if (stdinPipe) {
        NSData *input = [stdinString dataUsingEncoding:NSUTF8StringEncoding];
        [stdinPipe.fileHandleForWriting writeData:input];
        [stdinPipe.fileHandleForWriting closeFile];
    }

    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
        [task waitUntilExit];
        dispatch_semaphore_signal(semaphore);
    });

    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, (int64_t)(timeout * NSEC_PER_SEC));
    if (dispatch_semaphore_wait(semaphore, deadline) != 0) {
        [task terminate];
        CKMSetError(error, @"KeydockForCodex.Command", -2, @"Command timed out.");
        return nil;
    }

    NSData *stdoutData = [stdoutPipe.fileHandleForReading readDataToEndOfFile];
    NSData *stderrData = [stderrPipe.fileHandleForReading readDataToEndOfFile];

    CKMCommandResult *result = [[CKMCommandResult alloc] init];
    result.status = task.terminationStatus;
    result.standardOutput = [[NSString alloc] initWithData:stdoutData encoding:NSUTF8StringEncoding] ?: @"";
    result.standardError = [[NSString alloc] initWithData:stderrData encoding:NSUTF8StringEncoding] ?: @"";
    return result;
}

@end

@interface CKMCodexService : NSObject
+ (NSString *)findCodexPath:(NSError **)error;
+ (BOOL)loginWithAPIKey:(NSString *)apiKey codexPath:(NSString *)codexPath statusOutput:(NSString **)statusOutput error:(NSError **)error;
+ (BOOL)restartCodexDesktop:(NSError **)error;
@end

@implementation CKMCodexService

+ (NSString *)findCodexPath:(NSError **)error {
    NSString *shell = NSProcessInfo.processInfo.environment[@"SHELL"];
    if (shell.length == 0 || ![NSFileManager.defaultManager isExecutableFileAtPath:shell]) {
        shell = @"/bin/zsh";
    }

    NSError *commandError = nil;
    CKMCommandResult *result = [CKMCommandRunner runExecutable:shell
                                                    arguments:@[@"-lc", @"command -v codex"]
                                                  stdinString:nil
                                                      timeout:8
                                                        error:&commandError];
    NSString *path = CKMTrim(result.standardOutput);
    if (result && result.status == 0 && path.length > 0 && [NSFileManager.defaultManager isExecutableFileAtPath:path]) {
        return path;
    }

    NSMutableArray<NSString *> *fallbacks = [@[
        @"/opt/homebrew/bin/codex",
        @"/usr/local/bin/codex",
        [NSHomeDirectory() stringByAppendingPathComponent:@".local/bin/codex"],
        [NSHomeDirectory() stringByAppendingPathComponent:@".nvm/current/bin/codex"]
    ] mutableCopy];

    NSString *nvmNodeRoot = [NSHomeDirectory() stringByAppendingPathComponent:@".nvm/versions/node"];
    NSArray<NSString *> *nodeVersions = [NSFileManager.defaultManager contentsOfDirectoryAtPath:nvmNodeRoot error:nil];
    NSArray<NSString *> *sortedVersions = [nodeVersions sortedArrayUsingSelector:@selector(localizedStandardCompare:)];
    for (NSString *version in sortedVersions.reverseObjectEnumerator) {
        [fallbacks addObject:[[nvmNodeRoot stringByAppendingPathComponent:version] stringByAppendingPathComponent:@"bin/codex"]];
    }

    for (NSString *fallback in fallbacks) {
        if ([NSFileManager.defaultManager isExecutableFileAtPath:fallback]) {
            return fallback;
        }
    }

    NSString *detail = commandError.localizedDescription ?: @"Codex CLI was not found in the login shell PATH.";
    CKMSetError(error, @"KeydockForCodex.Codex", -1, [NSString stringWithFormat:@"%@ Configure codex in your shell first.", detail]);
    return nil;
}

+ (BOOL)loginWithAPIKey:(NSString *)apiKey codexPath:(NSString *)codexPath statusOutput:(NSString **)statusOutput error:(NSError **)error {
    NSString *input = [NSString stringWithFormat:@"%@\n", CKMTrim(apiKey)];
    CKMCommandResult *login = [CKMCommandRunner runExecutable:codexPath
                                                   arguments:@[@"login", @"--with-api-key"]
                                                 stdinString:input
                                                     timeout:30
                                                       error:error];
    if (!login) {
        return NO;
    }
    if (login.status != 0) {
        NSString *message = CKMTrim(login.standardError).length > 0 ? CKMTrim(login.standardError) : @"codex login --with-api-key failed.";
        return CKMSetError(error, @"KeydockForCodex.Codex", login.status, message);
    }

    CKMCommandResult *status = [CKMCommandRunner runExecutable:codexPath
                                                    arguments:@[@"login", @"status"]
                                                  stdinString:nil
                                                      timeout:15
                                                        error:error];
    if (!status) {
        return NO;
    }
    if (status.status != 0) {
        NSString *message = CKMTrim(status.standardError).length > 0 ? CKMTrim(status.standardError) : @"codex login status failed after switching.";
        return CKMSetError(error, @"KeydockForCodex.Codex", status.status, message);
    }

    if (statusOutput) {
        *statusOutput = CKMTrim(status.standardOutput);
    }
    return YES;
}

+ (BOOL)restartCodexDesktop:(NSError **)error {
    const char *disabled = getenv("CKM_DISABLE_RESTART");
    if (disabled && strcmp(disabled, "1") == 0) {
        return YES;
    }

    NSArray<NSRunningApplication *> *running = [NSRunningApplication runningApplicationsWithBundleIdentifier:CKMCodexBundleIdentifier];
    for (NSRunningApplication *application in running) {
        [application terminate];
    }

    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:6.0];
    while ([NSDate.date compare:deadline] == NSOrderedAscending) {
        NSArray<NSRunningApplication *> *remaining = [NSRunningApplication runningApplicationsWithBundleIdentifier:CKMCodexBundleIdentifier];
        if (remaining.count == 0) {
            break;
        }
        [NSThread sleepForTimeInterval:0.2];
    }

    CKMCommandResult *open = [CKMCommandRunner runExecutable:@"/usr/bin/open"
                                                   arguments:@[@"-b", CKMCodexBundleIdentifier]
                                                 stdinString:nil
                                                     timeout:10
                                                       error:error];
    if (!open) {
        return NO;
    }
    if (open.status != 0) {
        NSString *message = CKMTrim(open.standardError).length > 0 ? CKMTrim(open.standardError) : @"Unable to reopen Codex.app.";
        return CKMSetError(error, @"KeydockForCodex.Codex", open.status, message);
    }
    return YES;
}

@end

#ifndef CKM_TESTING

@interface CKMInsetLabel : NSTextField
@end

@implementation CKMInsetLabel

- (instancetype)initWithFrame:(NSRect)frameRect {
    self = [super initWithFrame:frameRect];
    if (self) {
        self.bezeled = NO;
        self.drawsBackground = NO;
        self.editable = NO;
        self.selectable = NO;
        self.font = [NSFont systemFontOfSize:13 weight:NSFontWeightRegular];
        self.textColor = NSColor.secondaryLabelColor;
    }
    return self;
}

@end

@interface CKMBackgroundView : NSView
@property (nonatomic, strong) NSColor *fillColor;
@end

@implementation CKMBackgroundView

- (void)drawRect:(NSRect)dirtyRect {
    [(self.fillColor ?: NSColor.windowBackgroundColor) setFill];
    NSRectFill(dirtyRect);
}

@end

@interface CKMAppDelegate : NSObject <NSApplicationDelegate, NSTableViewDataSource, NSTableViewDelegate, NSTextFieldDelegate>
@property (nonatomic, strong) NSWindow *window;
@property (nonatomic, strong) NSTableView *tableView;
@property (nonatomic, strong) NSTextField *titleField;
@property (nonatomic, strong) NSTextField *nameField;
@property (nonatomic, strong) NSTextField *maskedField;
@property (nonatomic, strong) NSTextField *statusField;
@property (nonatomic, strong) NSTextField *lastValidatedField;
@property (nonatomic, strong) NSTextField *messageField;
@property (nonatomic, strong) NSButton *saveNameButton;
@property (nonatomic, strong) NSButton *deleteButton;
@property (nonatomic, strong) NSButton *validateButton;
@property (nonatomic, strong) NSButton *switchButton;
@property (nonatomic, strong) NSProgressIndicator *progressIndicator;
@property (nonatomic, strong) CKMMetadataStore *store;
@property (nonatomic, strong) NSMutableArray<CKMKeyRecord *> *records;
@property (nonatomic, strong) CKMOpenAIValidator *validator;
@property (nonatomic, assign) BOOL busy;
@end

@implementation CKMAppDelegate

- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    (void)notification;
    self.store = [[CKMMetadataStore alloc] init];
    self.validator = [[CKMOpenAIValidator alloc] init];
    NSError *error = nil;
    self.records = [self.store loadRecordsWithError:&error] ?: [NSMutableArray array];
    [self buildMenu];
    [self buildWindow];
    [self.tableView reloadData];
    if (self.records.count > 0) {
        [self.tableView selectRowIndexes:[NSIndexSet indexSetWithIndex:0] byExtendingSelection:NO];
    }
    [self refreshDetails];
    if (error) {
        [self showError:error title:@"Unable to load keys"];
    }
}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)sender {
    (void)sender;
    return YES;
}

- (void)buildMenu {
    NSMenu *mainMenu = [[NSMenu alloc] initWithTitle:@"Main"];
    NSMenuItem *appItem = [[NSMenuItem alloc] initWithTitle:@"Keydock for Codex" action:nil keyEquivalent:@""];
    [mainMenu addItem:appItem];
    NSMenu *appMenu = [[NSMenu alloc] initWithTitle:@"Keydock for Codex"];
    [appMenu addItemWithTitle:@"Quit Keydock for Codex" action:@selector(terminate:) keyEquivalent:@"q"];
    appItem.submenu = appMenu;
    NSApp.mainMenu = mainMenu;
}

- (NSTextField *)labelWithText:(NSString *)text frame:(NSRect)frame font:(NSFont *)font color:(NSColor *)color {
    NSTextField *label = [[NSTextField alloc] initWithFrame:frame];
    label.stringValue = text ?: @"";
    label.bezeled = NO;
    label.drawsBackground = NO;
    label.editable = NO;
    label.selectable = NO;
    label.font = font;
    label.textColor = color;
    label.lineBreakMode = NSLineBreakByTruncatingTail;
    return label;
}

- (NSButton *)buttonWithTitle:(NSString *)title frame:(NSRect)frame action:(SEL)action {
    NSButton *button = [[NSButton alloc] initWithFrame:frame];
    button.title = title;
    button.bezelStyle = NSBezelStyleRounded;
    button.target = self;
    button.action = action;
    return button;
}

- (void)buildWindow {
    NSRect frame = NSMakeRect(0, 0, 920, 560);
    self.window = [[NSWindow alloc] initWithContentRect:frame
                                             styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable
                                               backing:NSBackingStoreBuffered
                                                 defer:NO];
    self.window.title = @"Keydock for Codex";
    self.window.minSize = NSMakeSize(760, 480);
    [self.window center];

    CKMBackgroundView *content = [[CKMBackgroundView alloc] initWithFrame:frame];
    content.fillColor = [NSColor colorWithCalibratedWhite:0.975 alpha:1.0];
    content.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    self.window.contentView = content;

    CGFloat sidebarWidth = 292.0;
    CKMBackgroundView *sidebar = [[CKMBackgroundView alloc] initWithFrame:NSMakeRect(0, 0, sidebarWidth, frame.size.height)];
    sidebar.fillColor = [NSColor colorWithCalibratedRed:0.925 green:0.935 blue:0.93 alpha:1.0];
    sidebar.autoresizingMask = NSViewHeightSizable;
    [content addSubview:sidebar];

    NSTextField *sidebarTitle = [self labelWithText:@"Stored keys"
                                              frame:NSMakeRect(20, frame.size.height - 52, 170, 26)
                                               font:[NSFont systemFontOfSize:20 weight:NSFontWeightSemibold]
                                              color:NSColor.labelColor];
    sidebarTitle.autoresizingMask = NSViewMinYMargin;
    [sidebar addSubview:sidebarTitle];

    NSButton *addButton = [self buttonWithTitle:@"+" frame:NSMakeRect(sidebarWidth - 58, frame.size.height - 54, 34, 30) action:@selector(addKey:)];
    addButton.font = [NSFont systemFontOfSize:18 weight:NSFontWeightSemibold];
    addButton.autoresizingMask = NSViewMinYMargin;
    [sidebar addSubview:addButton];

    NSScrollView *scrollView = [[NSScrollView alloc] initWithFrame:NSMakeRect(12, 20, sidebarWidth - 24, frame.size.height - 88)];
    scrollView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    scrollView.hasVerticalScroller = YES;
    scrollView.drawsBackground = NO;
    self.tableView = [[NSTableView alloc] initWithFrame:scrollView.bounds];
    self.tableView.headerView = nil;
    self.tableView.rowHeight = 58;
    self.tableView.delegate = self;
    self.tableView.dataSource = self;
    self.tableView.selectionHighlightStyle = NSTableViewSelectionHighlightStyleRegular;
    NSTableColumn *column = [[NSTableColumn alloc] initWithIdentifier:@"key"];
    column.width = scrollView.bounds.size.width;
    [self.tableView addTableColumn:column];
    scrollView.documentView = self.tableView;
    [sidebar addSubview:scrollView];

    NSView *detail = [[NSView alloc] initWithFrame:NSMakeRect(sidebarWidth, 0, frame.size.width - sidebarWidth, frame.size.height)];
    detail.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    [content addSubview:detail];

    self.titleField = [self labelWithText:@"Keydock for Codex"
                                    frame:NSMakeRect(36, frame.size.height - 64, 430, 32)
                                     font:[NSFont systemFontOfSize:26 weight:NSFontWeightBold]
                                    color:NSColor.labelColor];
    self.titleField.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    [detail addSubview:self.titleField];

    NSTextField *notice = [self labelWithText:@"Terminal Codex sessions must be reopened after switching."
                                        frame:NSMakeRect(38, frame.size.height - 92, 500, 22)
                                         font:[NSFont systemFontOfSize:13 weight:NSFontWeightRegular]
                                        color:NSColor.secondaryLabelColor];
    notice.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    [detail addSubview:notice];

    CGFloat left = 38;
    CGFloat top = frame.size.height - 144;
    [detail addSubview:[self labelWithText:@"Name" frame:NSMakeRect(left, top + 34, 120, 20) font:[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold] color:NSColor.secondaryLabelColor]];
    self.nameField = [[NSTextField alloc] initWithFrame:NSMakeRect(left, top, 360, 28)];
    self.nameField.font = [NSFont systemFontOfSize:14 weight:NSFontWeightRegular];
    self.nameField.delegate = self;
    self.nameField.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    [detail addSubview:self.nameField];

    self.saveNameButton = [self buttonWithTitle:@"Save name" frame:NSMakeRect(left + 374, top - 1, 104, 30) action:@selector(saveName:)];
    self.saveNameButton.autoresizingMask = NSViewMinXMargin | NSViewMinYMargin;
    [detail addSubview:self.saveNameButton];

    [detail addSubview:[self labelWithText:@"Masked key" frame:NSMakeRect(left, top - 46, 120, 20) font:[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold] color:NSColor.secondaryLabelColor]];
    self.maskedField = [self labelWithText:@"-" frame:NSMakeRect(left, top - 78, 360, 26) font:[NSFont monospacedSystemFontOfSize:14 weight:NSFontWeightRegular] color:NSColor.labelColor];
    self.maskedField.selectable = YES;
    self.maskedField.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    [detail addSubview:self.maskedField];

    [detail addSubview:[self labelWithText:@"Status" frame:NSMakeRect(left, top - 124, 120, 20) font:[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold] color:NSColor.secondaryLabelColor]];
    self.statusField = [self labelWithText:@"-" frame:NSMakeRect(left, top - 156, 300, 26) font:[NSFont systemFontOfSize:15 weight:NSFontWeightSemibold] color:NSColor.labelColor];
    self.statusField.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    [detail addSubview:self.statusField];

    [detail addSubview:[self labelWithText:@"Last checked" frame:NSMakeRect(left, top - 202, 160, 20) font:[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold] color:NSColor.secondaryLabelColor]];
    self.lastValidatedField = [self labelWithText:@"-" frame:NSMakeRect(left, top - 234, 360, 24) font:[NSFont systemFontOfSize:14 weight:NSFontWeightRegular] color:NSColor.labelColor];
    self.lastValidatedField.autoresizingMask = NSViewWidthSizable | NSViewMinYMargin;
    [detail addSubview:self.lastValidatedField];

    self.validateButton = [self buttonWithTitle:@"Check key" frame:NSMakeRect(left, 94, 108, 32) action:@selector(validateSelected:)];
    self.validateButton.autoresizingMask = NSViewMaxYMargin;
    [detail addSubview:self.validateButton];

    self.switchButton = [self buttonWithTitle:@"Switch & restart" frame:NSMakeRect(left + 120, 94, 148, 32) action:@selector(switchSelected:)];
    self.switchButton.bezelStyle = NSBezelStyleTexturedRounded;
    self.switchButton.autoresizingMask = NSViewMaxYMargin;
    [detail addSubview:self.switchButton];

    self.deleteButton = [self buttonWithTitle:@"Delete" frame:NSMakeRect(left + 280, 94, 92, 32) action:@selector(deleteSelected:)];
    self.deleteButton.autoresizingMask = NSViewMaxYMargin;
    [detail addSubview:self.deleteButton];

    self.progressIndicator = [[NSProgressIndicator alloc] initWithFrame:NSMakeRect(left, 52, 22, 22)];
    self.progressIndicator.style = NSProgressIndicatorStyleSpinning;
    self.progressIndicator.displayedWhenStopped = NO;
    self.progressIndicator.autoresizingMask = NSViewMaxYMargin;
    [detail addSubview:self.progressIndicator];

    self.messageField = [self labelWithText:@"Ready."
                                      frame:NSMakeRect(left + 32, 48, 520, 28)
                                       font:[NSFont systemFontOfSize:13 weight:NSFontWeightRegular]
                                      color:NSColor.secondaryLabelColor];
    self.messageField.autoresizingMask = NSViewWidthSizable | NSViewMaxYMargin;
    [detail addSubview:self.messageField];

    [self.window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
}

- (NSInteger)numberOfRowsInTableView:(NSTableView *)tableView {
    (void)tableView;
    return (NSInteger)self.records.count;
}

- (NSView *)tableView:(NSTableView *)tableView viewForTableColumn:(NSTableColumn *)tableColumn row:(NSInteger)row {
    (void)tableColumn;
    NSTableCellView *cell = [tableView makeViewWithIdentifier:@"KeyCell" owner:self];
    if (!cell) {
        cell = [[NSTableCellView alloc] initWithFrame:NSMakeRect(0, 0, tableView.bounds.size.width, 58)];
        cell.identifier = @"KeyCell";

        NSTextField *name = [self labelWithText:@""
                                          frame:NSMakeRect(12, 10, tableView.bounds.size.width - 24, 22)
                                           font:[NSFont systemFontOfSize:14 weight:NSFontWeightSemibold]
                                          color:NSColor.labelColor];
        name.tag = 101;
        name.autoresizingMask = NSViewWidthSizable;
        [cell addSubview:name];

        NSTextField *mask = [self labelWithText:@""
                                          frame:NSMakeRect(12, 32, tableView.bounds.size.width - 84, 18)
                                           font:[NSFont monospacedSystemFontOfSize:11 weight:NSFontWeightRegular]
                                          color:NSColor.secondaryLabelColor];
        mask.tag = 102;
        mask.autoresizingMask = NSViewWidthSizable;
        [cell addSubview:mask];

        NSTextField *active = [self labelWithText:@"ACTIVE"
                                            frame:NSMakeRect(tableView.bounds.size.width - 62, 31, 50, 18)
                                             font:[NSFont systemFontOfSize:10 weight:NSFontWeightBold]
                                            color:[NSColor colorWithCalibratedRed:0.02 green:0.44 blue:0.34 alpha:1.0]];
        active.alignment = NSTextAlignmentRight;
        active.tag = 103;
        active.autoresizingMask = NSViewMinXMargin;
        [cell addSubview:active];
    }

    CKMKeyRecord *record = self.records[(NSUInteger)row];
    NSTextField *name = [cell viewWithTag:101];
    NSTextField *mask = [cell viewWithTag:102];
    NSTextField *active = [cell viewWithTag:103];
    name.stringValue = record.label ?: @"Untitled key";
    mask.stringValue = record.maskedKey ?: @"";
    active.hidden = !record.active;
    return cell;
}

- (void)tableViewSelectionDidChange:(NSNotification *)notification {
    (void)notification;
    [self refreshDetails];
}

- (CKMKeyRecord *)selectedRecord {
    NSInteger row = self.tableView.selectedRow;
    if (row < 0 || row >= (NSInteger)self.records.count) {
        return nil;
    }
    return self.records[(NSUInteger)row];
}

- (void)refreshDetails {
    CKMKeyRecord *record = [self selectedRecord];
    BOOL hasRecord = record != nil;
    self.nameField.enabled = hasRecord && !self.busy;
    self.saveNameButton.enabled = hasRecord && !self.busy;
    self.deleteButton.enabled = hasRecord && !self.busy;
    self.validateButton.enabled = hasRecord && !self.busy;
    self.switchButton.enabled = hasRecord && !self.busy;

    if (!record) {
        self.nameField.stringValue = @"";
        self.maskedField.stringValue = @"-";
        self.statusField.stringValue = @"No key selected";
        self.statusField.textColor = NSColor.secondaryLabelColor;
        self.lastValidatedField.stringValue = @"-";
        return;
    }

    self.nameField.stringValue = record.label ?: @"Untitled key";
    self.maskedField.stringValue = record.maskedKey ?: @"";
    if (record.active) {
        self.statusField.stringValue = @"Active in Codex";
        self.statusField.textColor = [NSColor colorWithCalibratedRed:0.0 green:0.45 blue:0.33 alpha:1.0];
    } else if (record.lastValidatedAt.length > 0) {
        self.statusField.stringValue = @"Validated";
        self.statusField.textColor = NSColor.labelColor;
    } else {
        self.statusField.stringValue = @"Not checked";
        self.statusField.textColor = NSColor.secondaryLabelColor;
    }
    self.lastValidatedField.stringValue = CKMDisplayDate(record.lastValidatedAt);
}

- (void)setBusy:(BOOL)busy message:(NSString *)message {
    self.busy = busy;
    if (busy) {
        [self.progressIndicator startAnimation:nil];
    } else {
        [self.progressIndicator stopAnimation:nil];
    }
    self.messageField.stringValue = message ?: @"";
    [self refreshDetails];
}

- (void)showError:(NSError *)error title:(NSString *)title {
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = title ?: @"Error";
    alert.informativeText = error.localizedDescription ?: @"Unknown error";
    alert.alertStyle = NSAlertStyleWarning;
    [alert addButtonWithTitle:@"OK"];
    [alert beginSheetModalForWindow:self.window completionHandler:nil];
}

- (void)addKey:(id)sender {
    (void)sender;
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = @"Add API key";
    alert.informativeText = @"The key will be checked before it is saved.";
    [alert addButtonWithTitle:@"Add"];
    [alert addButtonWithTitle:@"Cancel"];

    NSView *accessory = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, 390, 92)];
    NSTextField *nameLabel = [self labelWithText:@"Name" frame:NSMakeRect(0, 68, 80, 20) font:[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold] color:NSColor.secondaryLabelColor];
    [accessory addSubview:nameLabel];
    NSTextField *nameField = [[NSTextField alloc] initWithFrame:NSMakeRect(84, 64, 300, 24)];
    nameField.stringValue = @"OpenAI key";
    [accessory addSubview:nameField];
    NSTextField *keyLabel = [self labelWithText:@"API key" frame:NSMakeRect(0, 30, 80, 20) font:[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold] color:NSColor.secondaryLabelColor];
    [accessory addSubview:keyLabel];
    NSSecureTextField *keyField = [[NSSecureTextField alloc] initWithFrame:NSMakeRect(84, 26, 300, 24)];
    [accessory addSubview:keyField];
    alert.accessoryView = accessory;

    NSModalResponse response = [alert runModal];
    if (response != NSAlertFirstButtonReturn) {
        return;
    }

    NSString *label = CKMTrim(nameField.stringValue);
    NSString *apiKey = CKMTrim(keyField.stringValue);
    if (apiKey.length == 0) {
        [self showError:[NSError errorWithDomain:@"KeydockForCodex.UI" code:0 userInfo:@{NSLocalizedDescriptionKey: @"Enter an API key first."}] title:@"Missing API key"];
        return;
    }

    [self setBusy:YES message:@"Checking key before saving..."];
    [self.validator validateKey:apiKey completion:^(CKMValidationResult *result) {
        dispatch_async(dispatch_get_main_queue(), ^{
            if (!result.valid) {
                [self setBusy:NO message:@"Key was not saved."];
                [self showError:[NSError errorWithDomain:@"KeydockForCodex.Validation" code:result.statusCode userInfo:@{NSLocalizedDescriptionKey: result.message ?: @"Validation failed."}] title:@"Key check failed"];
                return;
            }

            CKMKeyRecord *record = [CKMKeyRecord recordWithLabel:label apiKey:apiKey];
            NSError *keychainError = nil;
            if (![CKMKeychain saveSecret:apiKey account:record.identifier error:&keychainError]) {
                [self setBusy:NO message:@"Key was not saved."];
                [self showError:keychainError title:@"Keychain error"];
                return;
            }

            [self.records addObject:record];
            NSError *saveError = nil;
            if (![self.store saveRecords:self.records error:&saveError]) {
                [CKMKeychain deleteSecretForAccount:record.identifier error:nil];
                [self.records removeObject:record];
                [self setBusy:NO message:@"Key was not saved."];
                [self showError:saveError title:@"Unable to save metadata"];
                return;
            }

            [self.tableView reloadData];
            [self.tableView selectRowIndexes:[NSIndexSet indexSetWithIndex:self.records.count - 1] byExtendingSelection:NO];
            [self setBusy:NO message:@"Key saved and validated."];
        });
    }];
}

- (void)saveName:(id)sender {
    (void)sender;
    CKMKeyRecord *record = [self selectedRecord];
    if (!record) {
        return;
    }
    NSString *newName = CKMTrim(self.nameField.stringValue);
    if (newName.length == 0) {
        newName = @"Untitled key";
    }
    record.label = newName;
    record.updatedAt = CKMNowString();
    NSError *error = nil;
    if (![self.store saveRecords:self.records error:&error]) {
        [self showError:error title:@"Unable to save name"];
        return;
    }
    [self.tableView reloadData];
    [self refreshDetails];
    self.messageField.stringValue = @"Name saved.";
}

- (void)deleteSelected:(id)sender {
    (void)sender;
    CKMKeyRecord *record = [self selectedRecord];
    NSInteger row = self.tableView.selectedRow;
    if (!record) {
        return;
    }

    NSAlert *confirm = [[NSAlert alloc] init];
    confirm.messageText = @"Delete this key?";
    confirm.informativeText = @"The secret will also be removed from Keychain.";
    [confirm addButtonWithTitle:@"Delete"];
    [confirm addButtonWithTitle:@"Cancel"];
    confirm.alertStyle = NSAlertStyleWarning;
    if ([confirm runModal] != NSAlertFirstButtonReturn) {
        return;
    }

    NSError *keychainError = nil;
    if (![CKMKeychain deleteSecretForAccount:record.identifier error:&keychainError]) {
        [self showError:keychainError title:@"Keychain error"];
        return;
    }
    [self.records removeObjectAtIndex:(NSUInteger)row];
    NSError *saveError = nil;
    if (![self.store saveRecords:self.records error:&saveError]) {
        [self showError:saveError title:@"Unable to save metadata"];
        return;
    }
    [self.tableView reloadData];
    if (self.records.count > 0) {
        NSUInteger next = MIN((NSUInteger)row, self.records.count - 1);
        [self.tableView selectRowIndexes:[NSIndexSet indexSetWithIndex:next] byExtendingSelection:NO];
    }
    [self refreshDetails];
    self.messageField.stringValue = @"Key deleted.";
}

- (void)validateSelected:(id)sender {
    (void)sender;
    CKMKeyRecord *record = [self selectedRecord];
    if (!record) {
        return;
    }

    NSError *readError = nil;
    NSString *apiKey = [CKMKeychain readSecretForAccount:record.identifier error:&readError];
    if (!apiKey) {
        [self showError:readError title:@"Keychain error"];
        return;
    }

    [self setBusy:YES message:@"Checking key..."];
    [self.validator validateKey:apiKey completion:^(CKMValidationResult *result) {
        dispatch_async(dispatch_get_main_queue(), ^{
            if (result.valid) {
                record.lastValidatedAt = CKMNowString();
                record.updatedAt = CKMNowString();
                NSError *saveError = nil;
                [self.store saveRecords:self.records error:&saveError];
                [self.tableView reloadData];
                [self setBusy:NO message:@"Key is valid."];
                if (saveError) {
                    [self showError:saveError title:@"Unable to save validation time"];
                }
            } else {
                [self setBusy:NO message:result.message ?: @"Key check failed."];
            }
        });
    }];
}

- (void)switchSelected:(id)sender {
    (void)sender;
    CKMKeyRecord *record = [self selectedRecord];
    if (!record) {
        return;
    }

    NSError *readError = nil;
    NSString *apiKey = [CKMKeychain readSecretForAccount:record.identifier error:&readError];
    if (!apiKey) {
        [self showError:readError title:@"Keychain error"];
        return;
    }

    [self setBusy:YES message:@"Checking key before switching..."];
    [self.validator validateKey:apiKey completion:^(CKMValidationResult *result) {
        if (!result.valid) {
            dispatch_async(dispatch_get_main_queue(), ^{
                [self setBusy:NO message:@"Switch blocked because the key check failed."];
                [self showError:[NSError errorWithDomain:@"KeydockForCodex.Validation" code:result.statusCode userInfo:@{NSLocalizedDescriptionKey: result.message ?: @"Validation failed."}] title:@"Switch blocked"];
            });
            return;
        }

        dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
            NSError *error = nil;
            NSString *codexPath = [CKMCodexService findCodexPath:&error];
            NSString *statusOutput = nil;
            BOOL switched = NO;
            if (codexPath) {
                switched = [CKMCodexService loginWithAPIKey:apiKey codexPath:codexPath statusOutput:&statusOutput error:&error];
            }
            if (switched) {
                switched = [CKMCodexService restartCodexDesktop:&error];
            }

            dispatch_async(dispatch_get_main_queue(), ^{
                if (!switched) {
                    [self setBusy:NO message:@"Switch failed."];
                    [self showError:error title:@"Unable to switch key"];
                    return;
                }

                for (CKMKeyRecord *candidate in self.records) {
                    candidate.active = [candidate.identifier isEqualToString:record.identifier];
                    candidate.updatedAt = CKMNowString();
                }
                record.lastValidatedAt = CKMNowString();
                NSError *saveError = nil;
                [self.store saveRecords:self.records error:&saveError];
                [self.tableView reloadData];
                [self refreshDetails];
                NSString *message = statusOutput.length > 0 ? [NSString stringWithFormat:@"Switched. %@ Terminal Codex sessions must be reopened.", statusOutput] : @"Switched. Terminal Codex sessions must be reopened.";
                [self setBusy:NO message:message];
                if (saveError) {
                    [self showError:saveError title:@"Unable to save active key"];
                }
            });
        });
    }];
}

@end

int main(int argc, const char * argv[]) {
    (void)argc;
    (void)argv;
    @autoreleasepool {
        NSApplication *application = NSApplication.sharedApplication;
        application.activationPolicy = NSApplicationActivationPolicyRegular;
        CKMAppDelegate *delegate = [[CKMAppDelegate alloc] init];
        application.delegate = delegate;
        [application run];
    }
    return 0;
}

#endif
