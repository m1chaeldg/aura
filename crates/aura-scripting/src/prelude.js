// Embedded Zero-Dependency Faker Implementation
(function () {
    let seed = Date.now() ^ (Math.random() * 0x10000000);
    function pseudo() {
        seed = (seed * 9301 + 49297) % 233280;
        return seed / 233280;
    }
    function pick(arr) {
        return arr[Math.floor(pseudo() * arr.length)];
    }
    function randInt(min, max) {
        return Math.floor(pseudo() * (max - min + 1)) + min;
    }

    const firstNames = ["James", "Mary", "Robert", "Patricia", "John", "Jennifer", "Michael", "Linda", "David", "Elizabeth", "William", "Barbara", "Richard", "Susan", "Joseph", "Jessica", "Thomas", "Sarah"];
    const lastNames = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson"];
    const domains = ["example.com", "test.org", "demo.net", "aura.dev", "mail.com"];
    const cities = ["New York", "London", "Tokyo", "Paris", "Berlin", "Sydney", "Toronto", "Singapore", "San Francisco", "Dublin"];
    const countries = ["United States", "United Kingdom", "Canada", "Germany", "France", "Japan", "Australia", "Singapore"];
    const streets = ["Main St", "High St", "Maple Ave", "Oak St", "Park Blvd", "Broadway", "Market St", "Lakeview Dr"];
    const words = ["agile", "robust", "scalable", "ephemeral", "secure", "distributed", "protocol", "payload", "stream", "pipeline", "cluster", "gateway", "latency"];
    const companies = ["Acme Corp", "Globex", "Initech", "Umbrella", "Cyberdyne", "Soylent", "Massive Dynamic", "Hooli", "Pied Piper"];
    const jobTitles = ["Software Engineer", "DevOps Specialist", "Product Manager", "QA Architect", "Systems Analyst", "SRE"];

    function genUuid() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
            const r = Math.floor(pseudo() * 16);
            const v = c === 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    globalThis.faker = {
        person: {
            firstName: () => pick(firstNames),
            lastName: () => pick(lastNames),
            fullName: () => `${pick(firstNames)} ${pick(lastNames)}`,
            jobTitle: () => pick(jobTitles),
        },
        name: {
            firstName: () => pick(firstNames),
            lastName: () => pick(lastNames),
            fullName: () => `${pick(firstNames)} ${pick(lastNames)}`,
            findName: () => `${pick(firstNames)} ${pick(lastNames)}`,
            jobTitle: () => pick(jobTitles),
        },
        internet: {
            email: () => `${pick(firstNames).toLowerCase()}.${pick(lastNames).toLowerCase()}${randInt(10, 999)}@${pick(domains)}`,
            exampleEmail: () => `${pick(firstNames).toLowerCase()}${randInt(1, 99)}@example.com`,
            userName: () => `${pick(firstNames).toLowerCase()}_${randInt(100, 999)}`,
            password: (len = 12) => {
                const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%";
                return Array.from({ length: len }, () => chars[Math.floor(pseudo() * chars.length)]).join('');
            },
            url: () => `https://${pick(words)}.${pick(domains)}/api/v1`,
            ip: () => `${randInt(1, 254)}.${randInt(1, 254)}.${randInt(1, 254)}.${randInt(1, 254)}`,
            ipv6: () => `2001:db8:${randInt(1000, 9999).toString(16)}:${randInt(1000, 9999).toString(16)}::1`,
            color: () => `#${Math.floor(pseudo() * 16777215).toString(16).padStart(6, '0')}`,
        },
        number: {
            int: (opts) => {
                const max = typeof opts === 'number' ? opts : (opts?.max ?? 10000);
                const min = opts?.min ?? 0;
                return randInt(min, max);
            },
            float: (opts) => {
                const min = opts?.min ?? 0;
                const max = opts?.max ?? 100;
                const prec = opts?.precision ?? 0.01;
                const val = min + pseudo() * (max - min);
                return Math.round(val / prec) * prec;
            }
        },
        datatype: {
            uuid: () => genUuid(),
            boolean: () => pseudo() > 0.5,
            number: (max = 1000) => randInt(0, max),
        },
        random: {
            uuid: () => genUuid(),
            boolean: () => pseudo() > 0.5,
            number: (max = 1000) => randInt(0, max),
            word: () => pick(words),
            words: (count = 3) => Array.from({ length: count }, () => pick(words)).join(' '),
            alphaNumeric: (len = 8) => {
                const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
                return Array.from({ length: len }, () => chars[Math.floor(pseudo() * chars.length)]).join('');
            }
        },
        string: {
            uuid: () => genUuid(),
            alphanumeric: (len = 8) => {
                const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
                return Array.from({ length: len }, () => chars[Math.floor(pseudo() * chars.length)]).join('');
            },
            sample: (len = 10) => Array.from({ length: len }, () => pick(words)).join(' '),
        },
        location: {
            city: () => pick(cities),
            country: () => pick(countries),
            countryCode: () => "US",
            streetAddress: () => `${randInt(100, 9999)} ${pick(streets)}`,
            street: () => pick(streets),
        },
        address: {
            city: () => pick(cities),
            country: () => pick(countries),
            streetAddress: () => `${randInt(100, 9999)} ${pick(streets)}`,
        },
        company: {
            name: () => pick(companies),
            companyName: () => pick(companies),
        },
        commerce: {
            price: () => `${randInt(5, 500)}.${randInt(10, 99)}`,
            productName: () => `${pick(words)} ${pick(words)}`,
        },
        lorem: {
            word: () => pick(words),
            words: (count = 3) => Array.from({ length: count }, () => pick(words)).join(' '),
            sentence: () => `${pick(words)} ${pick(words)} ${pick(words)} ${pick(words)}.`,
        },
        date: {
            recent: () => new Date(),
            past: () => new Date(Date.now() - randInt(86400000, 86400000 * 30)),
            future: () => new Date(Date.now() + randInt(86400000, 86400000 * 30)),
            now: () => Date.now(),
        }
    };
})();

// Jasmine / Chai Style Testing Environment
(function () {
    let currentSuite = "";

    function fail(msg) {
        throw new Error(msg);
    }

    function deepEqual(a, b) {
        if (a === b) return true;
        if (a == null || b == null) return false;
        if (typeof a !== 'object' || typeof b !== 'object') return false;
        const keysA = Object.keys(a);
        const keysB = Object.keys(b);
        if (keysA.length !== keysB.length) return false;
        for (const k of keysA) {
            if (!keysB.includes(k) || !deepEqual(a[k], b[k])) return false;
        }
        return true;
    }

    function createExpectation(actual, isNegated = false) {
        const check = (passed, msg) => {
            const result = isNegated ? !passed : passed;
            if (!result) {
                fail(msg);
            }
        };

        return {
            get not() {
                return createExpectation(actual, !isNegated);
            },
            toBe(expected) {
                check(actual === expected, `Expected ${JSON.stringify(actual)} to be ${JSON.stringify(expected)}`);
            },
            toEqual(expected) {
                check(deepEqual(actual, expected), `Expected ${JSON.stringify(actual)} to equal ${JSON.stringify(expected)}`);
            },
            toBeDefined() {
                check(actual !== undefined, `Expected value to be defined, but was undefined`);
            },
            toBeUndefined() {
                check(actual === undefined, `Expected value to be undefined, but was ${JSON.stringify(actual)}`);
            },
            toBeNull() {
                check(actual === null, `Expected value to be null, but was ${JSON.stringify(actual)}`);
            },
            toBeTruthy() {
                check(Boolean(actual), `Expected ${JSON.stringify(actual)} to be truthy`);
            },
            toBeFalsy() {
                check(!Boolean(actual), `Expected ${JSON.stringify(actual)} to be falsy`);
            },
            toBeGreaterThan(n) {
                check(actual > n, `Expected ${actual} to be greater than ${n}`);
            },
            toBeLessThan(n) {
                check(actual < n, `Expected ${actual} to be less than ${n}`);
            },
            toBeGreaterThanOrEqual(n) {
                check(actual >= n, `Expected ${actual} to be >= ${n}`);
            },
            toBeLessThanOrEqual(n) {
                check(actual <= n, `Expected ${actual} to be <= ${n}`);
            },
            toContain(item) {
                if (typeof actual === 'string' || Array.isArray(actual)) {
                    check(actual.includes(item), `Expected ${JSON.stringify(actual)} to contain ${JSON.stringify(item)}`);
                } else {
                    fail(`Expected string or array for toContain, got ${typeof actual}`);
                }
            },
            toMatch(pattern) {
                const regex = pattern instanceof RegExp ? pattern : new RegExp(pattern);
                check(regex.test(String(actual)), `Expected ${JSON.stringify(actual)} to match ${pattern}`);
            },
            toThrow() {
                if (typeof actual !== 'function') fail('Expected function for toThrow assertion');
                let threw = false;
                try {
                    actual();
                } catch {
                    threw = true;
                }
                check(threw, `Expected function to throw error`);
            },
            // Chai BDD compatibility
            to: {
                equal(expected) {
                    check(actual === expected, `Expected ${JSON.stringify(actual)} to equal ${JSON.stringify(expected)}`);
                },
                deep: {
                    equal(expected) {
                        check(deepEqual(actual, expected), `Expected ${JSON.stringify(actual)} to deeply equal ${JSON.stringify(expected)}`);
                    }
                },
                be: {
                    a(expectedType) {
                        const actualType = typeof actual;
                        check(actualType === expectedType, `Expected type '${actualType}' to be '${expectedType}'`);
                    },
                    get true() {
                        check(actual === true, `Expected true, got ${JSON.stringify(actual)}`);
                    },
                    get false() {
                        check(actual === false, `Expected false, got ${JSON.stringify(actual)}`);
                    }
                },
                have: {
                    property(prop) {
                        check(actual != null && Object.prototype.hasOwnProperty.call(actual, prop), `Expected object to have property '${prop}'`);
                    }
                }
            }
        };
    }

    let testQueue = Promise.resolve();

    globalThis.expect = function (actual) {
        return createExpectation(actual, false);
    };

    globalThis.describe = function (suiteName, fn) {
        const prev = currentSuite;
        const suiteTitle = prev ? `${prev} > ${suiteName}` : String(suiteName);
        currentSuite = suiteTitle;
        try {
            const res = fn();
            if (res && typeof res.then === 'function') {
                const p = res.finally(() => {
                    currentSuite = prev;
                });
                testQueue = testQueue.then(() => p);
                return p;
            } else {
                currentSuite = prev;
                return res;
            }
        } catch (err) {
            currentSuite = prev;
            throw err;
        }
    };

    globalThis.it = function (testName, fn) {
        const fullTitle = currentSuite ? `${currentSuite} > ${testName}` : String(testName);
        const start = Date.now();
        const runTest = async () => {
            try {
                const res = fn();
                if (res && typeof res.then === 'function') {
                    await res;
                }
                globalThis.__aura_internal.assertions.push({
                    name: fullTitle,
                    passed: true,
                    error_message: null,
                    duration_ms: Date.now() - start
                });
            } catch (err) {
                globalThis.__aura_internal.assertions.push({
                    name: fullTitle,
                    passed: false,
                    error_message: (err && (err.message || err.stack)) ? String(err.message || err.stack) : String(err),
                    duration_ms: Date.now() - start
                });
            }
        };
        testQueue = testQueue.then(runTest);
        return testQueue;
    };

    globalThis.test = globalThis.it;

    globalThis.__aura_drain_tests = async function () {
        await testQueue;
    };
})();
