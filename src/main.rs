use clap::Parser;


#[derive(Parser)]
#[command(about = "Small tool to fix jellyfin issue 15272")]
struct Args {
    /// Path to the `jellyfin.db` database file.
    database_path: String,
}
fn main() {
    let Args {database_path} = Args::parse();

    println!("Fixing jellyfin issue 15272 for database at: {database_path}");

    // Use sqlite to open the database.
    let conn = sqlite::Connection::open(&database_path).expect("Failed to open database");
    println!("Database opened successfully.");


    let mut rows = Vec::new();
    conn.iterate("select Id, Name from BaseItems where DateCreated is null and AlbumArtists is not null", |row| {
        let id = row[0].1.expect("Null `Id` column is null");
        let album_name = row[1].1.expect("Null `Name` column is null");
        
        println!("Found album {album_name}");
        rows.push((id.to_string(), album_name.to_string()));

        true
    }).expect("Failed to iterate over rows");

    for (id, album_name) in rows {
        // Use the `Id` column to get all items with this as the parent id.
        let mut date_created = None;
        let r = conn.iterate(format!("select DateCreated from BaseItems where ParentId = '{id}'"), |child_row| {
            let child_date_created = child_row.get(0).map(|p| p.1).flatten();
            if let Some(date) = child_date_created {
                date_created = Some(date.to_owned());
                false
            } else {
                true
            }
        });

        if let Err(e) = r {
            // Ignore expected `abort` error when early exiting the date-search loop.
            if !matches!(e.code, Some(4)) {
                eprintln!("Failed to iterate over child rows");
                continue;
            }
        }

        // If we found a date created, update the parent item.
        if let Some(date) = date_created {
            conn.execute(format!("update BaseItems set DateCreated = '{date}' where Id = '{id}'")).expect("Failed to update DateCreated");
            println!("SUCCESS: Updated DateCreated for album: {album_name}");
        } else {
            eprintln!("FAILURE: No valid DateCreated found for album: {album_name}");
        }
    }
}
