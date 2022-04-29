(function () {
    var md = window.markdownit({
        html: true,
        linkify: true,
        typographer: true,
        highlight: function (str, lang) {
            if (lang && hljs.getLanguage(lang)) {
                try {
                    return '<pre class="hljs"><code>' +
                        hljs.highlight(lang, str, true).value +
                        '</code></pre>';
                } catch (__) {}
            }

            return '<pre class="hljs"><code>' + md.utils.escapeHtml(str) + '</code></pre>';
        }
    });

    function scrollToLint(lintId) {
        var target = document.getElementById(lintId);
        if (!target) {
            return;
        }
        target.scrollIntoView();
    }

    function scrollToLintByURL($scope) {
        var removeListener = $scope.$on('ngRepeatFinished', function(ngRepeatFinishedEvent) {
            scrollToLint(window.location.hash.slice(1));
            removeListener();
        });
    }

    function selectGroup($scope, selectedGroup) {
        var groups = $scope.groups;
        for (var group in groups) {
            if (groups.hasOwnProperty(group)) {
                if (group === selectedGroup) {
                    groups[group] = true;
                } else {
                    groups[group] = false;
                }
            }
        }
    }

    angular.module("clippy", [])
        .filter('markdown', function ($sce) {
            return function (text) {
                return $sce.trustAsHtml(
                    md.render(text || '')
                        // Oh deer, what a hack :O
                        .replace('<table', '<table class="table"')
                );
            };
        })
        .directive('themeDropdown', function ($document) {
            return {
                restrict: 'A',
                link: function ($scope, $element, $attr) {
                    $element.bind('click', function () {
                        $element.toggleClass('open');
                        $element.addClass('open-recent');
                    });

                    $document.bind('click', function () {
                        if (!$element.hasClass('open-recent')) {
                            $element.removeClass('open');
                        }
                        $element.removeClass('open-recent');
                    })
                }
            }
        })
        .directive('filterDropdown', function ($document) {
            return {
                restrict: 'A',
                link: function ($scope, $element, $attr) {
                    $element.bind('click', function (event) {
                        if (event.target.closest('button')) {
                            $element.toggleClass('open');
                        } else {
                            $element.addClass('open');
                        }
                        $element.addClass('open-recent');
                    });

                    $document.bind('click', function () {
                        if (!$element.hasClass('open-recent')) {
                            $element.removeClass('open');
                        }
                        $element.removeClass('open-recent');
                    })
                }
            }
        })
        .directive('onFinishRender', function ($timeout) {
            return {
                restrict: 'A',
                link: function (scope, element, attr) {
                    if (scope.$last === true) {
                        $timeout(function () {
                            scope.$emit(attr.onFinishRender);
                        });
                    }
                }
            };
        })
        .controller("lintList", function ($scope, $http, $timeout) {
            // Level filter
            var LEVEL_FILTERS_DEFAULT = {allow: true, warn: true, deny: true, none: true};
            $scope.levels = LEVEL_FILTERS_DEFAULT;
            $scope.byLevels = function (lint) {
                return $scope.levels[lint.level];
            };

            var GROUPS_FILTER_DEFAULT = {
                cargo: true,
                complexity: true,
                correctness: true,
                deprecated: false,
                nursery: true,
                pedantic: true,
                perf: true,
                restriction: true,
                style: true,
                suspicious: true,
            };
            $scope.groups = GROUPS_FILTER_DEFAULT;
            const THEMES_DEFAULT = {
                light: "Light",
                rust: "Rust",
                coal: "Coal",
                navy: "Navy",
                ayu: "Ayu"
            };
            $scope.themes = THEMES_DEFAULT;

            const DEFAULT_VERSION_FILTERS = {
                "≥": { enabled: false, version_str: "" },
                "≤": { enabled: false, version_str: "" },
                "=": { enabled: false, version_str: "" },
            };
            // Weird workaround to get a copy of the object
            $scope.version_filters = JSON.parse(JSON.stringify(DEFAULT_VERSION_FILTERS));
            $scope.version_regex = new RegExp('1\.\\d{2}\.\\d');

            $scope.selectTheme = function (theme) {
                setTheme(theme, true);
            }

            $scope.toggleLevels = function (value) {
                const levels = $scope.levels;
                for (const key in levels) {
                    if (levels.hasOwnProperty(key)) {
                        levels[key] = value;
                    }
                }
            };

            $scope.toggleGroups = function (value) {
                const groups = $scope.groups;
                for (const key in groups) {
                    if (groups.hasOwnProperty(key)) {
                        groups[key] = value;
                    }
                }
            };

            $scope.selectedValuesCount = function (obj) {
                return Object.values(obj).filter(x => x).length;
            }

            $scope.clearVersionFilters = function () {
                $scope.version_filters = JSON.parse(JSON.stringify(DEFAULT_VERSION_FILTERS));
            }

            $scope.versionFilterCount = function(obj) {
                return Object.values(obj).filter(x => x.enabled).length;
            }

            $scope.byVersion = function(lint) {
                function validate_version_str(ver) {
                    return ver.length === 6 && $scope.version_regex.test(ver);
                }

                function cmp_version(ver1, ver2, filter) {
                    // < 0: lint_version < version
                    // 0: equal
                    // > 0: lint_version > version
                    let result = ver1.localeCompare(ver2, undefined, {
                        numeric: true,
                        sensitivity: "base"
                    });

                    // "==" gets the highest priority, since all filters are inclusive
                    return (result === 0) || (filter === ">=" && result > 0) || (filter === "<=" && result < 0);
                }

                let filters = $scope.version_filters;

                // Strip the "pre " prefix for pre 1.29.0 lints
                let lint_version = lint.version.startsWith("pre ") ? lint.version.substring(4, lint.version.length) : lint.version;

                for (const filter in filters) {
                    let version_str = filters[filter].version_str;

                    // Skip the work for version strings with invalid lengths or characters
                    if (!validate_version_str(version_str)) {
                        filters[filter].enabled = false;
                        continue;
                    }

                    filters[filter].enabled = true;

                    let result = cmp_version(lint_version, version_str, filter);
                    if (result && filter === "==") {
                        return true;
                    } else if (!result) {
                        return false;
                    }

                    let cmp_filter;
                    if (filter === "≥") {
                        cmp_filter = "≤";
                    } else {
                        cmp_filter = "≥";
                    }

                    let cmp_version_str = filters[cmp_filter].version_str;
                    if (!filters[cmp_filter].enabled || !validate_version_str(cmp_version_str)) {
                        return true;
                    }

                    return cmp_version(lint_version, cmp_version_str, cmp_filter);
                }

                return true;
            }

            $scope.byGroups = function (lint) {
                return $scope.groups[lint.group];
            };

            $scope.bySearch = function (lint, index, array) {
                let searchStr = $scope.search;
                // It can be `null` I haven't missed this value
                if (searchStr == null || searchStr.length < 3) {
                    return true;
                }
                searchStr = searchStr.toLowerCase();

                // Search by id
                if (lint.id.indexOf(searchStr.replace("-", "_")) !== -1) {
                    return true;
                }

                // Search the description
                // The use of `for`-loops instead of `foreach` enables us to return early
                let terms = searchStr.split(" ");
                let docsLowerCase = lint.docs.toLowerCase();
                for (index = 0; index < terms.length; index++) {
                    // This is more likely and will therefor be checked first
                    if (docsLowerCase.indexOf(terms[index]) !== -1) {
                        continue;
                    }

                    if (lint.id.indexOf(terms[index]) !== -1) {
                        continue;
                    }

                    return false;
                }

                return true;
            }

            // Get data
            $scope.open = {};
            $scope.loading = true;
            // This will be used to jump into the source code of the version that this documentation is for.
            $scope.docVersion = window.location.pathname.split('/')[2] || "master";

            if (window.location.hash.length > 1) {
                $scope.search = window.location.hash.slice(1);
                $scope.open[window.location.hash.slice(1)] = true;
                scrollToLintByURL($scope);
            }

            $http.get('./lints.json')
                .success(function (data) {
                    $scope.data = data;
                    $scope.loading = false;

                    var selectedGroup = getQueryVariable("sel");
                    if (selectedGroup) {
                        selectGroup($scope, selectedGroup.toLowerCase());
                    }

                    scrollToLintByURL($scope);

                    setTimeout(function () {
                        var el = document.getElementById('filter-input');
                        if (el) { el.focus() }
                    }, 0);
                })
                .error(function (data) {
                    $scope.error = data;
                    $scope.loading = false;
                });

            window.addEventListener('hashchange', function () {
                // trigger re-render
                $timeout(function () {
                    $scope.levels = LEVEL_FILTERS_DEFAULT;
                    $scope.search = window.location.hash.slice(1);
                    $scope.open[window.location.hash.slice(1)] = true;

                    scrollToLintByURL($scope);
                });
                return true;
            }, false);
        });
})();

function getQueryVariable(variable) {
    var query = window.location.search.substring(1);
    var vars = query.split('&');
    for (var i = 0; i < vars.length; i++) {
        var pair = vars[i].split('=');
        if (decodeURIComponent(pair[0]) == variable) {
            return decodeURIComponent(pair[1]);
        }
    }
}

function setTheme(theme, store) {
    let enableHighlight = false;
    let enableNight = false;
    let enableAyu = false;

    if (theme == "ayu") {
        enableAyu = true;
    } else if (theme == "coal" || theme == "navy") {
        enableNight = true;
    } else if (theme == "rust") {
        enableHighlight = true;
    } else {
        enableHighlight = true;
        // this makes sure that an unknown theme request gets set to a known one
        theme = "light";
    }
    document.getElementsByTagName("body")[0].className = theme;

    document.getElementById("styleHighlight").disabled = !enableHighlight;
    document.getElementById("styleNight").disabled = !enableNight;
    document.getElementById("styleAyu").disabled = !enableAyu;

    if (store) {
        try {
            localStorage.setItem('clippy-lint-list-theme', theme);
        } catch (e) { }
    }
}

// loading the theme after the initial load
setTheme(localStorage.getItem('clippy-lint-list-theme'), false);
