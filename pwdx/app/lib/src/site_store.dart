// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import 'rust/api.dart' as rust;

/// Non-secret metadata about a site, so you do not have to remember the counter, profile and length per site.
/// Stored locally in plain JSON. It contains no passwords and no key material.
class SavedSite {
  SavedSite({
    required this.site,
    required this.user,
    this.counter = 1,
    this.profile = rust.ProfileKind.full,
    required this.length,
  });

  final String site;
  final String user;
  final int counter;
  final rust.ProfileKind profile;
  final int length;

  Map<String, Object> toJson() => {
    'site': site,
    'user': user,
    'counter': counter,
    'profile': profile.name,
    'length': length,
    'spec': 'v1',
  };

  static SavedSite? fromJson(Object? j) {
    if (j is! Map) return null;
    try {
      return SavedSite(
        site: j['site'] as String,
        user: j['user'] as String,
        counter: j['counter'] as int,
        profile: rust.ProfileKind.values.byName(j['profile'] as String),
        length: j['length'] as int,
      );
    } catch (_) {
      return null;
    }
  }

  String get label => '$site  ·  $user';
}

class SiteStore {
  Future<File> _file() async {
    final dir = await getApplicationSupportDirectory();
    return File('${dir.path}${Platform.pathSeparator}sites.json');
  }

  Future<List<SavedSite>> load() async {
    try {
      final f = await _file();
      if (!await f.exists()) return [];
      final list = jsonDecode(await f.readAsString());
      if (list is! List) return [];
      return list.map(SavedSite.fromJson).whereType<SavedSite>().toList();
    } catch (_) {
      return [];
    }
  }

  Future<void> upsert(SavedSite s) async {
    final all = await load();
    all.removeWhere((x) => x.site == s.site && x.user == s.user);
    all.add(s);
    all.sort((a, b) => a.label.compareTo(b.label));
    final f = await _file();
    await f.writeAsString(jsonEncode(all.map((e) => e.toJson()).toList()));
  }

  Future<void> remove(SavedSite s) async {
    final all = await load();
    all.removeWhere((x) => x.site == s.site && x.user == s.user);
    final f = await _file();
    await f.writeAsString(jsonEncode(all.map((e) => e.toJson()).toList()));
  }
}
