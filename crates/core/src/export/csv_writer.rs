//! Row encoders shared by buffered legacy exports and bounded caller-owned sinks.
use crate::CrawlReport;
use std::{
    collections::HashSet,
    io::{self, Write},
};

pub(crate) struct CsvRows<W: Write> {
    writer: csv::Writer<W>,
    columns: Vec<(String, String)>,
    header: bool,
}
impl<W: Write> CsvRows<W> {
    pub(crate) fn new(
        sink: W,
        report: Option<&CrawlReport>,
        extra: &[(String, String)],
    ) -> anyhow::Result<Self> {
        let mut columns = Vec::new();
        if let Some(counts) =
            report.and_then(|r| crate::reporting_visibility(&r.config, &r.pages, &r.issues))
        {
            for (name, value) in [
                ("Captured Pages", counts.captured_pages),
                ("Visible Pages", counts.visible_pages),
                ("Hidden Pages", counts.hidden_pages),
                ("Captured Issues", counts.captured_issues),
                ("Visible Issues", counts.visible_issues),
                ("Hidden Issues", counts.hidden_issues),
            ] {
                columns.push((name.to_owned(), value.to_string()));
            }
        }
        columns.extend(
            extra
                .iter()
                .map(|(name, value)| (name.clone(), super::csv_text(value))),
        );
        let mut names = HashSet::new();
        anyhow::ensure!(
            columns
                .iter()
                .all(|(name, _)| !name.is_empty() && names.insert(name.as_str())),
            "Duplicate or empty CSV context column"
        );
        Ok(Self {
            writer: csv::Writer::from_writer(sink),
            columns,
            header: true,
        })
    }
    pub(crate) fn write_record<I, T>(&mut self, row: I) -> csv::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<[u8]>,
    {
        for field in row {
            if self.header
                && self
                    .columns
                    .iter()
                    .any(|(name, _)| name.as_bytes() == field.as_ref())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "CSV context column collides with an original column",
                )
                .into());
            }
            self.writer.write_field(field)?;
        }
        for (name, value) in &self.columns {
            self.writer
                .write_field(if self.header { name } else { value })?;
        }
        self.writer.write_record(std::iter::empty::<&[u8]>())?;
        self.header = false;
        Ok(())
    }
    pub(crate) fn finish(mut self) -> anyhow::Result<W> {
        self.writer.flush()?;
        self.writer
            .into_inner()
            .map_err(|error| anyhow::anyhow!(error.error().to_string()))
    }
}
