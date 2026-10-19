// Copyright 2026 NavZip contributors. MPL-2.0.
// Included once from new_tab_page_handler.cc. Import is command-line opt-in.

void NewTabPageHandler::NavZipStartLegacyImport() {
  if (navzip_import_started_ || profile_->IsOffTheRecord())
    return;
  navzip_import_started_ = true;
  const auto path = base::CommandLine::ForCurrentProcess()->GetSwitchValuePath(
      "navzip-import");
  if (path.empty())
    return;
  static base::NoDestructor<std::set<base::FilePath>> importing;
  if (!importing->insert(profile_->GetPath()).second)
    return;
  navzip_import_guard_ = base::ScopedClosureRunner(base::BindOnce(
      [](base::FilePath profile_path) { importing->erase(profile_path); },
      profile_->GetPath()));
  base::ThreadPool::PostTaskAndReplyWithResult(
      FROM_HERE, {base::MayBlock(), base::TaskPriority::USER_VISIBLE},
      base::BindOnce(
          [](base::FilePath path) -> std::optional<base::DictValue> {
            std::string data;
            if (!base::ReadFileToStringWithMaxSize(path, &data,
                                                   32 * 1024 * 1024))
              return std::nullopt;
            auto value = base::JSONReader::Read(data, base::JSON_PARSE_RFC);
            if (!value || !value->is_dict())
              return std::nullopt;
            return std::move(*value).TakeDict();
          },
          path),
      base::BindOnce(&NewTabPageHandler::NavZipApplyLegacyImport,
                     weak_ptr_factory_.GetWeakPtr()));
}

void NewTabPageHandler::NavZipApplyLegacyImport(
    std::optional<base::DictValue> data) {
  if (!data || profile_->IsOffTheRecord())
    return;
  const auto* source = data->FindString("sourceId");
  const auto* bookmark_entries = data->FindList("bookmarks");
  const auto* visits = data->FindList("history");
  const auto* tab_entries = data->FindList("tabs");
  if (data->FindInt("schemaVersion") != 1 || !source || source->size() != 64 ||
      !bookmark_entries || bookmark_entries->size() > 100000 || !visits ||
      visits->size() > 500000 || !tab_entries || tab_entries->size() > 1000)
    return;

  // Validate the complete envelope before performing any writes.
  for (const auto& item : *bookmark_entries) {
    if (!item.is_dict())
      return;
    const auto& entry = item.GetDict();
    const auto* url = entry.FindString("url");
    const auto* id = entry.FindString("id");
    const auto* title = entry.FindString("title");
    const auto* folder = entry.FindString("folder");
    if (!url || !id || !title || !folder || title->size() > 16384 ||
        folder->size() > 4096 ||
        !base::Uuid::ParseCaseInsensitive(*id).is_valid() ||
        !GURL(*url).is_valid() || !GURL(*url).SchemeIsHTTPOrHTTPS() ||
        GURL(*url).has_username() || GURL(*url).has_password())
      return;
  }
  for (const auto& item : *visits) {
    if (!item.is_dict())
      return;
    const auto* url = item.GetDict().FindString("url");
    if (!url || !item.GetDict().FindString("title") ||
        !item.GetDict().FindDouble("visitedAt") || !GURL(*url).is_valid() ||
        !GURL(*url).SchemeIsHTTPOrHTTPS() || GURL(*url).has_username() ||
        GURL(*url).has_password())
      return;
  }
  for (const auto& item : *tab_entries) {
    if (!item.is_dict())
      return;
    const auto* url = item.GetDict().FindString("url");
    if (!url || (*url != "navzip://newtab" &&
                 (!GURL(*url).is_valid() || !GURL(*url).SchemeIsHTTPOrHTTPS() ||
                  GURL(*url).has_username() || GURL(*url).has_password())))
      return;
  }
  auto* model = BookmarkModelFactory::GetForBrowserContext(profile_);
  auto* history_service = HistoryServiceFactory::GetForProfile(
      profile_, ServiceAccessType::EXPLICIT_ACCESS);
  if (!model || !history_service)
    return;
  if (!model->loaded()) {
    if (++navzip_import_retries_ > 20)
      return;
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostDelayedTask(
        FROM_HERE,
        base::BindOnce(&NewTabPageHandler::NavZipApplyLegacyImport,
                       weak_ptr_factory_.GetWeakPtr(), std::move(data)),
        base::Milliseconds(250));
    return;
  }
  // UUIDs make bookmark retry safe even if the process stops before preferences
  // are flushed. Never write directly into Chromium's database files.
  std::set<base::Uuid> existing;
  std::vector<const bookmarks::BookmarkNode*> stack{model->root_node()};
  while (!stack.empty()) {
    const auto* node = stack.back();
    stack.pop_back();
    existing.insert(node->uuid());
    for (const auto& child : node->children())
      stack.push_back(child.get());
  }
  const auto* root = model->bookmark_bar_node();
  for (const auto& item : *bookmark_entries) {
    const auto& entry = item.GetDict();
    const auto id = base::Uuid::ParseCaseInsensitive(*entry.FindString("id"));
    if (!existing.insert(id).second)
      continue;
    const bookmarks::BookmarkNode* parent = root;
    const auto& folder = *entry.FindString("folder");
    if (!folder.empty()) {
      const auto title = base::UTF8ToUTF16(folder);
      const bookmarks::BookmarkNode* match = nullptr;
      for (const auto& child : root->children()) {
        if (child->is_folder() && child->GetTitle() == title) {
          match = child.get();
          break;
        }
      }
      parent = match ? match
                     : model->AddFolder(root, root->children().size(), title);
    }
    model->AddURL(parent, parent->children().size(),
                  base::UTF8ToUTF16(*entry.FindString("title")),
                  GURL(*entry.FindString("url")), nullptr, std::nullopt, id);
  }
  // Query existing timestamps for every URL on every retry. A completion flag
  // alone cannot provide crash-safe deduplication across two independent
  // stores.
  std::map<std::string, base::ListValue> by_url;
  for (const auto& visit : *visits)
    by_url[*visit.GetDict().FindString("url")].Append(visit.Clone());
  for (auto& [url, entries] : by_url) {
    history_service->QueryURLAndVisits(
        GURL(url), history::VisitQuery404sPolicy::kInclude404s,
        base::BindOnce(&NewTabPageHandler::NavZipImportVisits,
                       weak_ptr_factory_.GetWeakPtr(), GURL(url),
                       std::move(entries)),
        &navzip_import_tasks_);
  }
  auto* browser = webui::GetBrowserWindowInterface(web_contents_);
  const std::string marker = "tabs_" + *source;
  if (browser && !profile_->GetPrefs()
                      ->GetDict("navzip.migrations")
                      .FindBool(marker)
                      .value_or(false)) {
    // Set the tab marker first to avoid duplicate windows on repeated imports.
    // The exported backup remains available if a crash interrupts restoration.
    ScopedDictPrefUpdate update(profile_->GetPrefs(), "navzip.migrations");
    update->Set(marker, true);
    for (const auto& item : *tab_entries) {
      const auto& entry = item.GetDict();
      const auto& url = *entry.FindString("url");
      const auto* active_id = data->FindString("activeTabId");
      const auto* tab_id = entry.FindString("id");
      auto* contents = chrome::AddAndReturnTabAt(
          browser, GURL(url == "navzip://newtab" ? "chrome://newtab/" : url),
          -1, active_id && tab_id && *active_id == *tab_id, std::nullopt,
          entry.FindBool("pinned").value_or(false));
      if (contents)
        contents->SetAudioMuted(entry.FindBool("muted").value_or(false));
    }
    if (const auto* settings = data->FindDict("settings")) {
      if (const auto restore = settings->FindBool("restore_session"))
        profile_->GetPrefs()->SetInteger(prefs::kRestoreOnStartup,
                                         *restore ? 1 : 5);
      if (const auto* layout = settings->FindString("tab_layout")) {
        if (auto* controller =
                tabs::VerticalTabStripStateController::From(browser))
          controller->SetVerticalTabsEnabled(*layout == "Vertical");
      }
    }
  }
}

void NewTabPageHandler::NavZipImportVisits(
    GURL url,
    base::ListValue entries,
    history::QueryURLAndVisitsResult previous) {
  auto* history_service = HistoryServiceFactory::GetForProfile(
      profile_, ServiceAccessType::EXPLICIT_ACCESS);
  if (!history_service || profile_->IsOffTheRecord())
    return;
  std::set<base::Time> times;
  if (previous.success)
    for (const auto& visit : previous.visits)
      times.insert(visit.visit_time);
  for (const auto& item : entries) {
    const auto& entry = item.GetDict();
    const auto time =
        base::Time::FromSecondsSinceUnixEpoch(*entry.FindDouble("visitedAt"));
    if (times.insert(time).second) {
      history_service->AddPage(url, time,
                               history::SOURCE_OS_MIGRATION_IMPORTED);
      history_service->SetPageTitle(
          url, base::UTF8ToUTF16(*entry.FindString("title")));
    }
  }
}
