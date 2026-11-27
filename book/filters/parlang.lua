-- Pandoc Lua filter for the ParLang book, shared by the PDF (latex) and
-- EPUB (markdown intermediate, then epub3) builds.
--
-- Generic over every source file:
--   * a code block with an include="<repo-relative path>" attribute gets the
--     exact bytes of that file as its body (missing file = build error);
--   * latex: code blocks become fvextra verbatim environments (BookParCode for
--     `parlang`, BookCode otherwise); other formats keep them as code blocks;
--   * latex: BookParCode listings are highlighted (keywords from KEYWORDS in
--     crates/parlang/src/parser.rs, comments, string and char literals), and
--     each listing is round-trip checked against its original text;
--   * `mermaid` blocks become a "diagram omitted" note;
--   * links to local files or anchors become plain text, http(s)/mailto stay;
--   * "Table of Contents" sections are dropped;
--   * header identifiers are prefixed with the source file stem so they stay
--     unique across chapters;
--   * latex: inline code is split into several Code inlines, with
--     \allowbreak{} after runs of / . : , _ and a discouraged break
--     (\penalty50) inside CamelCase words;
--   * latex: a slash in prose allows a line break after it;
--   * latex: tables whose columns all have default widths get relative widths
--     proportional to their longest cell text, so pandoc emits p-columns.
-- Code block text is never altered: ParLang highlighting only adds markup, and
-- a self-check proves that stripping it gives back the original text. Inline
-- code keeps its text and only gains break points. The source path comes from
-- the `book-source` metadata set by book/build-book.

local source = "book"
local stem = "doc"
local is_latex = FORMAT:match("latex") ~= nil

local function read_file(path)
  local handle = io.open(path, "rb")
  if not handle then
    error("parlang.lua: include file not found: " .. path)
  end
  local text = handle:read("a")
  handle:close()
  return (text:gsub("\n$", ""))
end

-- latex: ParLang listings (fence tag parlang) are highlighted by a minimal
-- lexer. The keywords come from KEYWORDS in crates/parlang/src/parser.rs,
-- read at build time, so there is no second keyword list. The lexer only
-- knows comments (--, nested {- -}, and the # annotations the docs use),
-- string and char literals, and identifiers; everything else stays plain.
local keyword_source = "crates/parlang/src/parser.rs"
local keywords = nil

local function load_keywords()
  local text = read_file(keyword_source)
  local list = text:match("pub const KEYWORDS: &%[&str%] = &%[(.-)%];")
  if not list then
    error("parlang.lua: KEYWORDS not found in " .. keyword_source)
  end
  local set, count = {}, 0
  for word in list:gmatch('"([%a_]+)"') do
    set[word] = true
    count = count + 1
  end
  if count < 10 or not set["let"] then
    error("parlang.lua: implausible KEYWORDS list in " .. keyword_source)
  end
  return set
end

local function lex(text)
  local tokens = {}
  local function push(class, s)
    tokens[#tokens + 1] = { class, s }
  end
  local i, n = 1, #text
  while i <= n do
    local c = text:sub(i, i)
    local two = text:sub(i, i + 1)
    if two == "{-" then
      local depth, j = 1, i + 2
      while j <= n and depth > 0 do
        local t = text:sub(j, j + 1)
        if t == "{-" then
          depth, j = depth + 1, j + 2
        elseif t == "-}" then
          depth, j = depth - 1, j + 2
        else
          j = j + 1
        end
      end
      push("com", text:sub(i, j - 1))
      i = j
    elseif two == "--" or c == "#" then
      local j = text:find("\n", i, true) or (n + 1)
      push("com", text:sub(i, j - 1))
      i = j
    elseif c == '"' then
      local j = i + 1
      while j <= n do
        local d = text:sub(j, j)
        if d == "\\" then
          j = j + 2
        elseif d == '"' then
          j = j + 1
          break
        elseif d == "\n" then
          break
        else
          j = j + 1
        end
      end
      j = math.min(j, n + 1)
      push("str", text:sub(i, j - 1))
      i = j
    elseif c == "'" then
      local lit = text:match("^'\\.'", i)
      if not lit then
        local ch = text:match("^'(" .. utf8.charpattern .. ")'", i)
        if ch and ch ~= "'" and ch ~= "\\" then
          lit = "'" .. ch .. "'"
        end
      end
      if lit then
        push("str", lit)
        i = i + #lit
      else
        push("txt", c)
        i = i + 1
      end
    else
      local word = text:match("^[%a_\128-\255][%w_\128-\255]*", i)
      if word then
        push(keywords[word] and "kw" or "txt", word)
        i = i + #word
      else
        push("txt", c)
        i = i + 1
      end
    end
  end
  return tokens
end

local function escape(s)
  return (s:gsub("[\\{}]", { ["\\"] = "\\PLbs{}", ["{"] = "\\PLlb{}", ["}"] = "\\PLrb{}" }))
end

local macros = { kw = "\\PLkw", com = "\\PLcom", str = "\\PLstr" }

-- Whitespace stays outside the macro groups: fancyvrb works line by line
-- and fvextra breaks long lines at spaces.
local function render(tokens)
  local out = {}
  for _, tok in ipairs(tokens) do
    local macro = macros[tok[1]]
    if macro then
      for space, word in tok[2]:gmatch("(%s*)(%S*)") do
        out[#out + 1] = space
        if word ~= "" then
          out[#out + 1] = macro .. "{" .. escape(word) .. "}"
        end
      end
    else
      out[#out + 1] = escape(tok[2])
    end
  end
  return table.concat(out)
end

local wrappers = { PLkw = true, PLcom = true, PLstr = true }
local escapes = { PLbs = "\\", PLlb = "{", PLrb = "}" }

-- Strips the highlighting markup again; nil if the markup is malformed.
local function strip(s)
  local out, depth, i = {}, 0, 1
  while i <= #s do
    local c = s:sub(i, i)
    if c == "\\" then
      local name = s:match("^\\(%a+)", i)
      if name and escapes[name] and s:sub(i + #name + 1, i + #name + 2) == "{}" then
        out[#out + 1] = escapes[name]
        i = i + #name + 3
      elseif name and wrappers[name] and s:sub(i + #name + 1, i + #name + 1) == "{" then
        depth = depth + 1
        i = i + #name + 2
      else
        return nil
      end
    elseif c == "}" then
      if depth == 0 then
        return nil
      end
      depth = depth - 1
      i = i + 1
    elseif c == "{" then
      return nil
    else
      out[#out + 1] = c
      i = i + 1
    end
  end
  if depth ~= 0 then
    return nil
  end
  return table.concat(out)
end

-- Highlights one listing and checks that stripping the markup gives back
-- exactly the original text.
local function highlight(text)
  keywords = keywords or load_keywords()
  local body = render(lex(text))
  if strip(body) ~= text then
    error("parlang.lua: highlighting would change a ParLang listing in " .. source)
  end
  return body
end

local function omitted_diagram()
  if is_latex then
    return pandoc.RawBlock("latex", "\\BookOmittedDiagram{" .. source .. "}")
  end
  local note = "Diagram omitted in this edition; see " .. source .. " in the repository."
  return pandoc.Para({ pandoc.Emph({ pandoc.Str(note) }) })
end

local function code_block(el)
  local include = el.attributes["include"]
  if include then
    el.text = read_file(include)
    el.attributes["include"] = nil
  end
  local lang = el.classes[1] or ""
  if lang == "mermaid" then
    return omitted_diagram()
  end
  if not is_latex then
    return el
  end
  local label = lang:gsub("[^%w%-]", "")
  local env = "BookCode"
  local opts = ""
  local body = el.text
  if label == "parlang" then
    env = "BookParCode"
    body = highlight(el.text)
  elseif label ~= "" then
    opts = "[label={" .. label .. "}]"
  end
  local latex = "\\begin{" .. env .. "}" .. opts .. "\n" .. body .. "\n\\end{" .. env .. "}"
  return pandoc.RawBlock("latex", latex)
end

local function link(el)
  local target = el.target
  if target:match("^https?://") or target:match("^mailto:") then
    return el
  end
  return el.content
end

local function header(el)
  if el.identifier ~= "" then
    el.identifier = stem .. "-" .. el.identifier
  end
  return el
end

local function is_toc_heading(block)
  if block.t ~= "Header" then
    return false
  end
  local title = pandoc.utils.stringify(block.content):lower()
  return title == "contents" or title:find("table of contents", 1, true) ~= nil
end

local function drop_toc(blocks)
  local out = pandoc.Blocks({})
  local skip_level = nil
  for _, block in ipairs(blocks) do
    if skip_level and block.t == "Header" and block.level <= skip_level then
      skip_level = nil
    end
    if skip_level == nil and is_toc_heading(block) then
      skip_level = block.level
    elseif skip_level == nil then
      out:insert(block)
    end
  end
  return out
end

-- latex: split inline code into several Code inlines so long paths and
-- identifiers can break at the line end: freely after runs of / . : , _
-- (\allowbreak{}), reluctantly inside CamelCase words (\penalty50).
-- Pandoc still escapes each piece and builds heading bookmarks from their
-- text.
local function breakable_code(el)
  local out = pandoc.Inlines({})
  local function piece(text, sep)
    if #out > 0 then
      out:insert(pandoc.RawInline("latex", sep))
    end
    out:insert(pandoc.Code(text, el.attr))
  end
  for run in el.text:gmatch("[^/.:,_]*[/.:,_]*") do
    if run ~= "" then
      local start = 1
      local sep = "\\allowbreak{}"
      for pos in run:gmatch("%l()%u") do
        piece(run:sub(start, pos - 1), sep)
        sep = "\\penalty50\\relax{}"
        start = pos
      end
      piece(run:sub(start), sep)
    end
  end
  if #out < 2 then
    return nil
  end
  return out
end

-- latex: a slash in prose (Church/Scott, or between code spans such as
-- Lam/Let/LetRec) allows a line break after it.
local function breakable_slash(el)
  if not el.text:find("/", 1, true) then
    return nil
  end
  local out = pandoc.Inlines({})
  for run in el.text:gmatch("[^/]*/*") do
    if run ~= "" then
      out:insert(pandoc.Str(run))
      if run:sub(-1) == "/" then
        out:insert(pandoc.RawInline("latex", "\\allowbreak{}"))
      end
    end
  end
  return out
end

local function default_width(width)
  return type(width) ~= "number" or width <= 0
end

local function cell_length(cell)
  local text = pandoc.utils.stringify(cell.contents)
  return utf8.len(text) or #text
end

local function measure_rows(rows, lengths)
  for _, row in ipairs(rows or {}) do
    for j, cell in ipairs(row.cells) do
      lengths[j] = math.max(lengths[j] or 0, cell_length(cell))
    end
  end
end

-- latex: tables whose columns all have default widths get relative widths
-- proportional to their longest cell text (clamped to 6..60 characters),
-- so pandoc emits wrapping p-columns instead of l-columns.
local function table_widths(tbl)
  for _, spec in ipairs(tbl.colspecs) do
    if not default_width(spec[2]) then
      return nil
    end
  end
  local lengths = {}
  measure_rows(tbl.head and tbl.head.rows, lengths)
  for _, body in ipairs(tbl.bodies or {}) do
    measure_rows(body.head, lengths)
    measure_rows(body.body, lengths)
  end
  measure_rows(tbl.foot and tbl.foot.rows, lengths)
  local total = 0
  local widths = {}
  for j = 1, #tbl.colspecs do
    widths[j] = math.min(math.max(lengths[j] or 0, 6), 60)
    total = total + widths[j]
  end
  if total == 0 then
    return nil
  end
  local specs = {}
  for j, spec in ipairs(tbl.colspecs) do
    specs[j] = { spec[1], widths[j] / total }
  end
  tbl.colspecs = specs
  return tbl
end

function Pandoc(doc)
  local meta_source = doc.meta["book-source"]
  if meta_source then
    source = pandoc.utils.stringify(meta_source)
    stem = source:gsub("^.*/", ""):gsub("%.md$", ""):gsub("[^%w]", "-")
  end
  doc.blocks = drop_toc(doc.blocks)
  local filter = {
    CodeBlock = code_block,
    Link = link,
    Header = header,
  }
  if is_latex then
    filter.Code = breakable_code
    filter.Str = breakable_slash
    filter.Table = table_widths
  end
  return doc:walk(filter)
end
